#include "sntp.hpp"

#include <netdb.h>
#include <poll.h>
#include <sys/socket.h>
#include <unistd.h>

#include <algorithm>
#include <cerrno>
#include <charconv>
#include <cstring>
#include <format>
#include <memory>

namespace gtavi::sntp {

namespace {

std::uint32_t read_be32(const std::uint8_t* p) {
    return (std::uint32_t{p[0]} << 24) | (std::uint32_t{p[1]} << 16) | (std::uint32_t{p[2]} << 8) | p[3];
}

void write_be32(std::uint8_t* p, std::uint32_t v) {
    p[0] = std::uint8_t(v >> 24);
    p[1] = std::uint8_t(v >> 16);
    p[2] = std::uint8_t(v >> 8);
    p[3] = std::uint8_t(v);
}

// Closes the socket when it goes out of scope.
struct Socket {
    int fd = -1;
    explicit Socket(int fd) : fd(fd) {}
    Socket(const Socket&) = delete;
    Socket& operator=(const Socket&) = delete;
    ~Socket() {
        if (fd >= 0) ::close(fd);
    }
};

struct AddrInfoDeleter {
    void operator()(addrinfo* info) const { ::freeaddrinfo(info); }
};
using AddrInfo = std::unique_ptr<addrinfo, AddrInfoDeleter>;

struct HostPort {
    std::string host;
    std::string port;
};

// Splits "host:port", accepting bracketed IPv6 literals.
HostPort split_host_port(std::string_view server) {
    auto cut = server.rfind(':');
    int port = 0;
    if (cut == std::string_view::npos || cut == 0) throw Error(ErrorKind::resolve, std::format("could not resolve {}", server));
    auto digits = server.substr(cut + 1);
    auto [end, ec] = std::from_chars(digits.data(), digits.data() + digits.size(), port);
    if (ec != std::errc{} || end != digits.data() + digits.size() || port < 1 || port > 65535)
        throw Error(ErrorKind::resolve, std::format("could not resolve {}", server));
    auto host = server.substr(0, cut);
    while (!host.empty() && (host.front() == '[' || host.front() == ']')) host.remove_prefix(1);
    while (!host.empty() && (host.back() == '[' || host.back() == ']')) host.remove_suffix(1);
    return {std::string{host}, std::to_string(port)};
}

AddrInfo resolve_server(std::string_view server) {
    auto [host, port] = split_host_port(server);
    addrinfo hints{};
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_DGRAM;
    hints.ai_protocol = IPPROTO_UDP;
    addrinfo* result = nullptr;
    if (::getaddrinfo(host.c_str(), port.c_str(), &hints, &result) != 0 || result == nullptr)
        throw Error(ErrorKind::resolve, std::format("could not resolve {}", server));
    return AddrInfo{result};
}

[[noreturn]] void network_error(int err) {
    throw Error(ErrorKind::network, std::format("network error: {}", std::strerror(err)));
}

// Performs one exchange with the server at `address`.
Measurement exchange(const addrinfo& address, std::string_view server, std::chrono::milliseconds timeout) {
    using clock = std::chrono::steady_clock;
    Socket socket{::socket(address.ai_family, SOCK_DGRAM | SOCK_CLOEXEC, IPPROTO_UDP)};
    if (socket.fd < 0) network_error(errno);
    // Connecting filters out datagrams from any other source address.
    if (::connect(socket.fd, address.ai_addr, address.ai_addrlen) != 0) network_error(errno);

    auto t1 = system_unix_nanos();
    auto started = clock::now();
    // The request carries the truncated NTP form of t1; the server echoes it.
    auto sent = Timestamp::from_unix_nanos(t1);
    auto request = encode_request(sent);
    if (::send(socket.fd, request.data(), request.size(), 0) < 0) network_error(errno);

    std::array<std::uint8_t, 512> buffer{};
    while (true) {
        auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(timeout - (clock::now() - started));
        if (remaining.count() <= 0) throw Error(ErrorKind::timeout, "the time server did not answer");
        pollfd pfd{socket.fd, POLLIN, 0};
        int ready = ::poll(&pfd, 1, static_cast<int>(std::max<std::int64_t>(1, remaining.count())));
        if (ready < 0) {
            if (errno == EINTR) continue;
            network_error(errno);
        }
        if (ready == 0) throw Error(ErrorKind::timeout, "the time server did not answer");
        auto received = ::recv(socket.fd, buffer.data(), buffer.size(), 0);
        auto elapsed = clock::now() - started;
        if (received < 0) {
            if (errno == EINTR || errno == EAGAIN) continue;
            network_error(errno);
        }
        Reply reply;
        try {
            reply = decode_reply(std::span{buffer.data(), static_cast<std::size_t>(received)}, sent);
        } catch (const Error& e) {
            // A stray or delayed datagram: keep waiting for the real reply
            // until the deadline passes.
            if (e.kind() == ErrorKind::origin_mismatch) continue;
            throw;
        }
        auto t4 = t1 + std::chrono::duration_cast<std::chrono::nanoseconds>(elapsed).count();
        auto [offset, delay] = offset_and_delay(t1, reply.receive.to_unix_nanos(), reply.transmit.to_unix_nanos(), t4);
        return {offset / 1e6, delay / 1e6, reply.stratum, std::string{server}};
    }
}

}  // namespace

std::int64_t system_unix_nanos() {
    using namespace std::chrono;
    return duration_cast<nanoseconds>(system_clock::now().time_since_epoch()).count();
}

Timestamp Timestamp::from_unix_nanos(std::int64_t nanos) {
    auto secs = nanos / nanos_per_second;
    auto sub = nanos % nanos_per_second;
    if (sub < 0) {
        secs -= 1;
        sub += nanos_per_second;
    }
    secs += ntp_unix_offset_seconds;
    return {static_cast<std::uint32_t>(secs),
            static_cast<std::uint32_t>((static_cast<std::uint64_t>(sub) << 32) / nanos_per_second)};
}

std::int64_t Timestamp::to_unix_nanos() const {
    std::int64_t secs = seconds;
    if ((seconds & 0x8000'0000u) == 0) secs += std::int64_t{1} << 32;
    auto nanos = (std::uint64_t{fraction} * nanos_per_second) >> 32;
    return (secs - ntp_unix_offset_seconds) * nanos_per_second + static_cast<std::int64_t>(nanos);
}

Timestamp Timestamp::read(std::span<const std::uint8_t, 8> bytes) {
    return {read_be32(bytes.data()), read_be32(bytes.data() + 4)};
}

void Timestamp::write(std::span<std::uint8_t, 8> bytes) const {
    write_be32(bytes.data(), seconds);
    write_be32(bytes.data() + 4, fraction);
}

Packet encode_request(Timestamp transmit) {
    Packet packet{};
    // Leap indicator 0 (no warning), version 4, mode 3 (client).
    packet[0] = (4 << 3) | 3;
    transmit.write(std::span<std::uint8_t, 8>{packet.data() + 40, 8});
    return packet;
}

Reply decode_reply(std::span<const std::uint8_t> packet, Timestamp sent) {
    if (packet.size() < packet_length)
        throw Error(ErrorKind::truncated, std::format("truncated reply ({} bytes)", packet.size()));
    auto at = [&](std::size_t offset) { return Timestamp::read(packet.subspan(offset).first<8>()); };
    int leap = packet[0] >> 6;
    int mode = packet[0] & 0b111;
    auto stratum = packet[1];
    if (mode != 4) throw Error(ErrorKind::unexpected_mode, std::format("unexpected packet mode {}", mode));
    if (stratum == 0) {
        std::string code(reinterpret_cast<const char*>(packet.data() + 12), 4);
        // Keep the code printable, as an ASCII decoder would.
        for (auto& c : code)
            if (static_cast<unsigned char>(c) > 0x7f) c = '?';
        throw Error(ErrorKind::kiss_of_death, std::format("the server refused the request ({})", code), code);
    }
    auto transmit = at(40);
    if (leap == 3 || stratum > 15 || (transmit.seconds == 0 && transmit.fraction == 0))
        throw Error(ErrorKind::unsynchronised, "the server clock is not synchronised");
    if (at(24) != sent) throw Error(ErrorKind::origin_mismatch, "the reply does not match the request");
    return {stratum, at(32), transmit};
}

OffsetDelay offset_and_delay(std::int64_t t1, std::int64_t t2, std::int64_t t3, std::int64_t t4) {
    auto offset = ((t2 - t1) + (t3 - t4)) / 2;
    auto delay = (t4 - t1) - (t3 - t2);
    return {offset, std::max<std::int64_t>(delay, 0)};
}

Measurement query(std::string_view server, std::chrono::milliseconds timeout, int samples) {
    auto addresses = resolve_server(server);
    std::optional<Measurement> best;
    std::optional<Error> last_error;
    for (int i = 0; i < std::max(1, samples); ++i) {
        try {
            auto m = exchange(*addresses, server, timeout);
            if (!best || m.round_trip_ms < best->round_trip_ms) best = std::move(m);
        } catch (const Error& e) {
            last_error = e;
            // Do not keep waiting on a server that has stopped answering.
            if (!best) break;
        }
    }
    if (best) return *best;
    if (last_error) throw *last_error;
    throw Error(ErrorKind::no_servers, "no time servers are configured");
}

Measurement measure(const std::vector<std::string>& servers, std::chrono::milliseconds timeout, int samples) {
    std::optional<Error> last;
    for (const auto& server : servers) {
        try {
            return query(server, timeout, samples);
        } catch (const Error& e) {
            last = e;
        }
    }
    if (!last) throw Error(ErrorKind::no_servers, "no time servers are configured");
    throw Error(ErrorKind::all_servers_failed, std::format("no time server could be reached: {}", last->what()),
                std::nullopt, last->kind());
}

}  // namespace gtavi::sntp
