#include <gtest/gtest.h>

#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>

#include <format>
#include <thread>

#include "sntp.hpp"

using namespace gtavi::sntp;
using namespace std::chrono_literals;

namespace {

constexpr std::int64_t ms = 1'000'000;

Packet reply_for(Timestamp sent) {
    Packet packet{};
    packet[0] = (4 << 3) | 4;
    packet[1] = 2;
    sent.write(std::span<std::uint8_t, 8>{packet.data() + 24, 8});
    Timestamp{3'000'000'000u, 1}.write(std::span<std::uint8_t, 8>{packet.data() + 32, 8});
    Timestamp{3'000'000'000u, 2}.write(std::span<std::uint8_t, 8>{packet.data() + 40, 8});
    return packet;
}

ErrorKind error_of(std::span<const std::uint8_t> packet, Timestamp sent) {
    try {
        decode_reply(packet, sent);
    } catch (const Error& e) {
        return e.kind();
    }
    ADD_FAILURE() << "decode_reply accepted the packet";
    return ErrorKind::network;
}

// A UDP socket bound to an ephemeral loopback port.
struct LoopbackSocket {
    int fd;
    int port;

    LoopbackSocket() : fd(::socket(AF_INET, SOCK_DGRAM, 0)) {
        sockaddr_in address{};
        address.sin_family = AF_INET;
        address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
        ::bind(fd, reinterpret_cast<sockaddr*>(&address), sizeof address);
        socklen_t length = sizeof address;
        ::getsockname(fd, reinterpret_cast<sockaddr*>(&address), &length);
        port = ntohs(address.sin_port);
    }
    LoopbackSocket(const LoopbackSocket&) = delete;
    ~LoopbackSocket() { ::close(fd); }

    std::string address() const { return std::format("127.0.0.1:{}", port); }
};

// An NTP server on the loopback interface whose clock is `skew` nanoseconds
// ahead of the system clock. It answers `replies` requests, then stops.
class FakeServer {
public:
    FakeServer(std::int64_t skew, int replies)
        : thread_([this, skew, replies] {
              for (int i = 0; i < replies; ++i) {
                  std::array<std::uint8_t, 512> request{};
                  sockaddr_in from{};
                  socklen_t length = sizeof from;
                  auto received = ::recvfrom(socket_.fd, request.data(), request.size(), 0,
                                             reinterpret_cast<sockaddr*>(&from), &length);
                  if (received < static_cast<ssize_t>(packet_length)) continue;
                  auto now = Timestamp::from_unix_nanos(system_unix_nanos() + skew);
                  Packet reply{};
                  reply[0] = (4 << 3) | 4;
                  reply[1] = 1;
                  std::copy_n(request.data() + 40, 8, reply.data() + 24);
                  now.write(std::span<std::uint8_t, 8>{reply.data() + 32, 8});
                  now.write(std::span<std::uint8_t, 8>{reply.data() + 40, 8});
                  ::sendto(socket_.fd, reply.data(), reply.size(), 0, reinterpret_cast<sockaddr*>(&from), length);
              }
          }) {}
    FakeServer(const FakeServer&) = delete;
    ~FakeServer() { thread_.join(); }

    std::string address() const { return socket_.address(); }

private:
    LoopbackSocket socket_;  // Declared before the thread so it exists when the thread starts.
    std::thread thread_;
};

}  // namespace

TEST(Sntp, RequestHasVersion4ClientHeader) {
    auto packet = encode_request({0x0102'0304, 0x0506'0708});
    EXPECT_EQ(packet[0], 0x23);
    for (std::size_t i = 1; i < 40; ++i) EXPECT_EQ(packet[i], 0) << i;
    EXPECT_EQ((std::vector<std::uint8_t>(packet.begin() + 40, packet.end())),
              (std::vector<std::uint8_t>{1, 2, 3, 4, 5, 6, 7, 8}));
}

TEST(Sntp, TimestampRoundTripsWithinANanosecond) {
    // 2026-11-19T00:00:00.123456789Z
    std::int64_t nanos = 1'795'046'400 * nanos_per_second + 123'456'789;
    auto back = Timestamp::from_unix_nanos(nanos).to_unix_nanos();
    EXPECT_LE(std::abs(back - nanos), 1);
}

TEST(Sntp, UnixEpochMapsToTheKnownNtpSecond) {
    auto ts = Timestamp::from_unix_nanos(0);
    EXPECT_EQ(ts, (Timestamp{2'208'988'800u, 0}));
    EXPECT_EQ(ts.to_unix_nanos(), 0);
}

TEST(Sntp, EraRolloverIn2036IsHandled) {
    // 2036-02-07T06:28:16Z is the first second of NTP era 1.
    std::int64_t rollover = 2'085'978'496 * nanos_per_second;
    auto ts = Timestamp::from_unix_nanos(rollover);
    EXPECT_EQ(ts.seconds, 0u);
    EXPECT_EQ(ts.to_unix_nanos(), rollover);
    auto before = Timestamp::from_unix_nanos(rollover - nanos_per_second);
    EXPECT_EQ(before.seconds, UINT32_MAX);
    EXPECT_EQ(before.to_unix_nanos(), rollover - nanos_per_second);
}

TEST(Sntp, NegativeInstantsRoundDownToTheEarlierSecond) {
    auto ts = Timestamp::from_unix_nanos(-1);
    EXPECT_EQ(ts.seconds, 2'208'988'799u);
    EXPECT_GE(ts.to_unix_nanos(), -2);
    EXPECT_LE(ts.to_unix_nanos(), 0);
}

TEST(Sntp, OffsetAndDelayFollowTheRfcFormula) {
    // Local clock 250 ms slow; 40 ms out, 5 ms in the server, 60 ms back.
    auto t1 = 1'000 * ms;
    auto t2 = t1 + 250 * ms + 40 * ms;
    auto t3 = t2 + 5 * ms;
    auto t4 = t1 + 105 * ms;
    auto [offset, delay] = offset_and_delay(t1, t2, t3, t4);
    // The asymmetric path (40 out, 60 back) biases the estimate by 10 ms.
    EXPECT_EQ(offset, 240 * ms);
    EXPECT_EQ(delay, 100 * ms);
}

TEST(Sntp, SymmetricPathGivesTheExactOffset) {
    auto t1 = 5'000 * ms;
    auto t2 = t1 - 1'500 * ms + 20 * ms;
    auto t3 = t2 + ms;
    auto t4 = t1 + 41 * ms;
    auto [offset, delay] = offset_and_delay(t1, t2, t3, t4);
    EXPECT_EQ(offset, -1'500 * ms);
    EXPECT_EQ(delay, 40 * ms);
}

TEST(Sntp, ValidReplyIsDecoded) {
    Timestamp sent{42, 7};
    auto packet = reply_for(sent);
    auto reply = decode_reply(packet, sent);
    EXPECT_EQ(reply.stratum, 2);
    EXPECT_EQ(reply.receive.fraction, 1u);
    EXPECT_EQ(reply.transmit.fraction, 2u);
}

TEST(Sntp, InvalidRepliesAreRejected) {
    Timestamp sent{42, 7};
    auto good = reply_for(sent);

    EXPECT_EQ(error_of(std::span{good}.first(47), sent), ErrorKind::truncated);

    auto client_mode = good;
    client_mode[0] = (4 << 3) | 3;
    EXPECT_EQ(error_of(client_mode, sent), ErrorKind::unexpected_mode);

    auto kiss = good;
    kiss[1] = 0;
    std::copy_n("RATE", 4, kiss.begin() + 12);
    try {
        decode_reply(kiss, sent);
        ADD_FAILURE() << "kiss-o'-death accepted";
    } catch (const Error& e) {
        EXPECT_EQ(e.kind(), ErrorKind::kiss_of_death);
        EXPECT_EQ(e.kiss_code(), "RATE");
    }

    auto alarm = good;
    alarm[0] |= 0b1100'0000;
    EXPECT_EQ(error_of(alarm, sent), ErrorKind::unsynchronised);

    auto no_time = good;
    std::fill_n(no_time.begin() + 40, 8, 0);
    EXPECT_EQ(error_of(no_time, sent), ErrorKind::unsynchronised);

    EXPECT_EQ(error_of(good, Timestamp{43, 7}), ErrorKind::origin_mismatch);
}

TEST(Sntp, MeasuresAKnownSkewOverLoopback) {
    FakeServer server{3'210 * ms, 3};
    auto m = query(server.address(), 2s, 3);
    EXPECT_NEAR(m.offset_ms, 3'210, 50);
    EXPECT_LT(m.round_trip_ms, 500);
    EXPECT_EQ(m.stratum, 1);
    EXPECT_EQ(m.server, server.address());
}

TEST(Sntp, FallsThroughToTheNextServer) {
    LoopbackSocket dead;
    FakeServer live{-750 * ms, 1};
    auto m = measure({dead.address(), live.address()}, 200ms, 1);
    EXPECT_EQ(m.server, live.address());
    EXPECT_NEAR(m.offset_ms, -750, 50);
}

TEST(Sntp, ReportsFailureWhenNothingAnswers) {
    LoopbackSocket dead;
    try {
        measure({dead.address()}, 100ms, 1);
        ADD_FAILURE() << "measure succeeded";
    } catch (const Error& e) {
        EXPECT_EQ(e.kind(), ErrorKind::all_servers_failed);
        EXPECT_EQ(e.cause(), ErrorKind::timeout);
        EXPECT_STREQ(e.what(), "no time server could be reached: the time server did not answer");
    }
    try {
        measure({}, 100ms, 1);
        ADD_FAILURE() << "measure succeeded";
    } catch (const Error& e) {
        EXPECT_EQ(e.kind(), ErrorKind::no_servers);
    }
}

TEST(Sntp, ReportsUnresolvableServers) {
    try {
        query("no-port-here", 100ms, 1);
        ADD_FAILURE() << "query succeeded";
    } catch (const Error& e) {
        EXPECT_EQ(e.kind(), ErrorKind::resolve);
    }
}
