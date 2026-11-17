#pragma once
// A small SNTP client (RFC 4330) on POSIX sockets.
//
// It answers one question: by how much is the system clock wrong? It sends a
// client-mode NTP packet, validates the reply and applies the standard
// four-timestamp calculation:
//
//   offset = ((t2 - t1) + (t3 - t4)) / 2
//   delay  =  (t4 - t1) - (t3 - t2)
//
// where t1 and t4 are the local send and receive times and t2 and t3 are the
// server's receive and transmit times. The elapsed time between t1 and t4 is
// taken from a monotonic clock, so a clock step during the exchange cannot
// corrupt the delay.

#include <array>
#include <chrono>
#include <cstdint>
#include <optional>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace gtavi::sntp {

// Size of an NTP packet without extension fields or authentication.
inline constexpr std::size_t packet_length = 48;
// Seconds between the NTP epoch (1900-01-01) and the Unix epoch (1970-01-01).
inline constexpr std::int64_t ntp_unix_offset_seconds = 2'208'988'800;
inline constexpr std::int64_t nanos_per_second = 1'000'000'000;

using Packet = std::array<std::uint8_t, packet_length>;

// A 64-bit NTP timestamp: whole seconds and a binary fraction of a second.
struct Timestamp {
    std::uint32_t seconds = 0;
    std::uint32_t fraction = 0;
    bool operator==(const Timestamp&) const = default;

    // The seconds field wraps every 2^32 seconds, as the protocol defines.
    static Timestamp from_unix_nanos(std::int64_t nanos);
    // Values with the top bit clear are taken to lie in era 1 (from
    // 7 February 2036), which keeps the conversion correct from 1968 to 2104.
    std::int64_t to_unix_nanos() const;

    static Timestamp read(std::span<const std::uint8_t, 8> bytes);
    void write(std::span<std::uint8_t, 8> bytes) const;
};

// The fields of a validated server reply that the calculation needs.
struct Reply {
    std::uint8_t stratum;
    Timestamp receive;   // When the request arrived at the server (t2).
    Timestamp transmit;  // When the reply left the server (t3).
};

// The result of one successful exchange with a time server.
struct Measurement {
    double offset_ms;      // Milliseconds to add to the system clock to obtain server time.
    double round_trip_ms;  // Network delay, excluding server processing.
    std::uint8_t stratum;  // 1 is a primary reference, 2 to 15 are secondary.
    std::string server;    // As passed by the caller.
};

enum class ErrorKind {
    resolve,             // The host name did not resolve.
    network,             // A socket operation failed.
    timeout,             // The server did not answer in time.
    truncated,           // The reply was shorter than an NTP header.
    unexpected_mode,     // The reply was not a server-mode packet.
    kiss_of_death,       // The server declined to serve time (stratum 0).
    unsynchronised,      // The server is not synchronised to a reference clock.
    origin_mismatch,     // The reply does not echo the timestamp that was sent.
    all_servers_failed,  // Every configured server failed.
    no_servers,          // No servers were configured.
};

// Why a measurement could not be taken. Messages are lower-case fragments,
// like the Rust original.
class Error : public std::runtime_error {
public:
    Error(ErrorKind kind, const std::string& message, std::optional<std::string> kiss_code = std::nullopt,
          std::optional<ErrorKind> cause = std::nullopt)
        : std::runtime_error(message), kind_(kind), kiss_code_(std::move(kiss_code)), cause_(cause) {}
    ErrorKind kind() const noexcept { return kind_; }
    // The four-character code of a kiss-o'-death reply.
    const std::optional<std::string>& kiss_code() const noexcept { return kiss_code_; }
    // For all_servers_failed: the kind of the last server's failure.
    std::optional<ErrorKind> cause() const noexcept { return cause_; }

private:
    ErrorKind kind_;
    std::optional<std::string> kiss_code_;
    std::optional<ErrorKind> cause_;
};

// The system clock as nanoseconds since the Unix epoch.
std::int64_t system_unix_nanos();

// Builds a client request carrying `transmit` as its transmit timestamp.
Packet encode_request(Timestamp transmit);

// Parses and validates a reply to a request sent with timestamp `sent`.
// Throws Error when the reply is unusable.
Reply decode_reply(std::span<const std::uint8_t> packet, Timestamp sent);

struct OffsetDelay {
    std::int64_t offset;
    std::int64_t delay;
};

// The SNTP on-wire calculation. Arguments and results are nanoseconds.
OffsetDelay offset_and_delay(std::int64_t t1, std::int64_t t2, std::int64_t t3, std::int64_t t4);

// Measures the clock offset against one server given as "host:port". Up to
// `samples` exchanges are made and the one with the shortest round trip is
// returned: the less time a packet spends in transit, the smaller the possible
// asymmetry between its two legs, and asymmetry is the error.
Measurement query(std::string_view server, std::chrono::milliseconds timeout, int samples);

// Tries each server in order and returns the first successful measurement.
Measurement measure(const std::vector<std::string>& servers, std::chrono::milliseconds timeout, int samples);

}  // namespace gtavi::sntp
