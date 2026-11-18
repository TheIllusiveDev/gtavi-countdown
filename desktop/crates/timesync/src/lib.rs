//! A small SNTP client (RFC 4330) built on the standard library only.
//!
//! The crate answers one question: by how much is the system clock wrong?
//! It sends a client-mode NTP packet, validates the reply and applies the
//! standard four-timestamp calculation:
//!
//! ```text
//! offset = ((t2 - t1) + (t3 - t4)) / 2
//! delay  =  (t4 - t1) - (t3 - t2)
//! ```
//!
//! where `t1` and `t4` are the local send and receive times and `t2` and `t3`
//! are the server's receive and transmit times. The elapsed time between `t1`
//! and `t4` is taken from a monotonic clock, so a clock step during the
//! exchange cannot corrupt the delay.

use std::fmt;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Size of an NTP packet without extension fields or authentication.
pub const PACKET_LEN: usize = 48;

/// Seconds between the NTP epoch (1900-01-01) and the Unix epoch (1970-01-01).
const NTP_UNIX_OFFSET_SECS: i64 = 2_208_988_800;

const NANOS_PER_SEC: i64 = 1_000_000_000;

/// The result of one successful exchange with a time server.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// Milliseconds to add to the system clock to obtain server time.
    pub offset_ms: f64,
    /// Round-trip network delay in milliseconds, excluding server processing.
    pub round_trip_ms: f64,
    /// Stratum of the server: 1 is a primary reference, 2 to 15 are secondary.
    pub stratum: u8,
    /// The host the measurement was taken from, as passed by the caller.
    pub server: String,
}

/// Why a measurement could not be taken.
#[derive(Debug)]
pub enum Error {
    /// The host name did not resolve to any address.
    Resolve(String),
    /// A socket operation failed or timed out.
    Io(io::Error),
    /// The reply was shorter than an NTP header.
    Truncated(usize),
    /// The reply was not a server-mode packet.
    UnexpectedMode(u8),
    /// The server declined to serve time (stratum 0, "kiss-o'-death").
    KissOfDeath([u8; 4]),
    /// The server is not synchronised to a reference clock.
    Unsynchronised,
    /// The reply does not echo the timestamp that was sent; it may be forged
    /// or belong to an earlier request.
    OriginMismatch,
    /// Every configured server failed. Holds the last failure.
    AllServersFailed(Box<Error>),
    /// No servers were configured.
    NoServers,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Resolve(host) => write!(f, "could not resolve {host}"),
            Error::Io(error) if is_timeout(error) => write!(f, "the time server did not answer"),
            Error::Io(error) => write!(f, "network error: {error}"),
            Error::Truncated(len) => write!(f, "truncated reply ({len} bytes)"),
            Error::UnexpectedMode(mode) => write!(f, "unexpected packet mode {mode}"),
            Error::KissOfDeath(code) => {
                write!(f, "the server refused the request ({})", String::from_utf8_lossy(code))
            }
            Error::Unsynchronised => write!(f, "the server clock is not synchronised"),
            Error::OriginMismatch => write!(f, "the reply does not match the request"),
            Error::AllServersFailed(last) => write!(f, "no time server could be reached: {last}"),
            Error::NoServers => write!(f, "no time servers are configured"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(error) => Some(error),
            Error::AllServersFailed(last) => Some(last.as_ref()),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut)
}

/// A 64-bit NTP timestamp: whole seconds and a binary fraction of a second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    pub seconds: u32,
    pub fraction: u32,
}

impl Timestamp {
    /// Converts nanoseconds since the Unix epoch to an NTP timestamp.
    /// The seconds field wraps every 2^32 seconds, as the protocol defines.
    pub fn from_unix_nanos(nanos: i64) -> Self {
        let secs = nanos.div_euclid(NANOS_PER_SEC) + NTP_UNIX_OFFSET_SECS;
        let sub = nanos.rem_euclid(NANOS_PER_SEC) as u64;
        Timestamp {
            seconds: secs as u32,
            fraction: ((sub << 32) / NANOS_PER_SEC as u64) as u32,
        }
    }

    /// Converts to nanoseconds since the Unix epoch.
    ///
    /// The 32-bit seconds field is ambiguous across eras. Values with the top
    /// bit clear are taken to lie in era 1 (from 7 February 2036), which keeps
    /// the conversion correct from 1968 until 2104.
    pub fn to_unix_nanos(self) -> i64 {
        let mut secs = i64::from(self.seconds);
        if self.seconds & 0x8000_0000 == 0 {
            secs += 1 << 32;
        }
        let nanos = (u64::from(self.fraction) * NANOS_PER_SEC as u64) >> 32;
        (secs - NTP_UNIX_OFFSET_SECS) * NANOS_PER_SEC + nanos as i64
    }

    fn read(bytes: &[u8]) -> Self {
        Timestamp {
            seconds: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            fraction: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        }
    }

    fn write(self, bytes: &mut [u8]) {
        bytes[..4].copy_from_slice(&self.seconds.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.fraction.to_be_bytes());
    }
}

/// Builds a client request carrying `transmit` as its transmit timestamp.
pub fn encode_request(transmit: Timestamp) -> [u8; PACKET_LEN] {
    let mut packet = [0u8; PACKET_LEN];
    // Leap indicator 0 (no warning), version 4, mode 3 (client).
    packet[0] = (4 << 3) | 3;
    transmit.write(&mut packet[40..48]);
    packet
}

/// The fields of a validated server reply that the calculation needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reply {
    pub stratum: u8,
    /// When the request arrived at the server (`t2`).
    pub receive: Timestamp,
    /// When the reply left the server (`t3`).
    pub transmit: Timestamp,
}

/// Parses and validates a reply to a request sent with timestamp `sent`.
pub fn decode_reply(packet: &[u8], sent: Timestamp) -> Result<Reply, Error> {
    if packet.len() < PACKET_LEN {
        return Err(Error::Truncated(packet.len()));
    }
    let leap = packet[0] >> 6;
    let mode = packet[0] & 0b111;
    let stratum = packet[1];
    if mode != 4 {
        return Err(Error::UnexpectedMode(mode));
    }
    if stratum == 0 {
        return Err(Error::KissOfDeath([packet[12], packet[13], packet[14], packet[15]]));
    }
    let transmit = Timestamp::read(&packet[40..48]);
    if leap == 3 || stratum > 15 || (transmit.seconds == 0 && transmit.fraction == 0) {
        return Err(Error::Unsynchronised);
    }
    if Timestamp::read(&packet[24..32]) != sent {
        return Err(Error::OriginMismatch);
    }
    Ok(Reply { stratum, receive: Timestamp::read(&packet[32..40]), transmit })
}

/// Applies the SNTP on-wire calculation. All arguments are nanoseconds since
/// the Unix epoch; the result is `(offset, delay)` in nanoseconds.
pub fn offset_and_delay(t1: i64, t2: i64, t3: i64, t4: i64) -> (i64, i64) {
    let offset = ((t2 - t1) + (t3 - t4)) / 2;
    let delay = (t4 - t1) - (t3 - t2);
    (offset, delay.max(0))
}

fn system_unix_nanos() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(since) => since.as_nanos() as i64,
        // A clock set before 1970 is still a valid input to the calculation.
        Err(before) => -(before.duration().as_nanos() as i64),
    }
}

/// Performs one exchange with the server at `address`.
fn exchange(address: SocketAddr, server: &str, timeout: Duration) -> Result<Measurement, Error> {
    let bind: SocketAddr = if address.is_ipv4() {
        ([0, 0, 0, 0], 0).into()
    } else {
        ([0u16; 8], 0).into()
    };
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(timeout))?;
    socket.set_write_timeout(Some(timeout))?;
    // Connecting filters out datagrams from any other source address.
    socket.connect(address)?;

    let t1 = system_unix_nanos();
    let started = Instant::now();
    // The request carries the truncated NTP form of t1; the server echoes it.
    let sent = Timestamp::from_unix_nanos(t1);
    socket.send(&encode_request(sent))?;

    let mut buffer = [0u8; 512];
    let deadline = started + timeout;
    loop {
        let received = socket.recv(&mut buffer)?;
        let elapsed = started.elapsed();
        match decode_reply(&buffer[..received], sent) {
            Ok(reply) => {
                let t4 = t1 + elapsed.as_nanos() as i64;
                let (offset, delay) = offset_and_delay(
                    t1,
                    reply.receive.to_unix_nanos(),
                    reply.transmit.to_unix_nanos(),
                    t4,
                );
                return Ok(Measurement {
                    offset_ms: offset as f64 / 1e6,
                    round_trip_ms: delay as f64 / 1e6,
                    stratum: reply.stratum,
                    server: server.to_owned(),
                });
            }
            // A stray or delayed datagram: keep waiting for the real reply
            // until the deadline passes.
            Err(Error::OriginMismatch) if Instant::now() < deadline => continue,
            Err(error) => return Err(error),
        }
    }
}

/// Measures the clock offset against one server, given as `host:port`.
///
/// Up to `samples` exchanges are made and the one with the shortest round
/// trip is returned: the less time a packet spends in transit, the smaller
/// the possible asymmetry between its two legs, and asymmetry is the error.
pub fn query(server: &str, timeout: Duration, samples: usize) -> Result<Measurement, Error> {
    let address = server
        .to_socket_addrs()
        .map_err(|_| Error::Resolve(server.to_owned()))?
        .next()
        .ok_or_else(|| Error::Resolve(server.to_owned()))?;

    let mut best: Option<Measurement> = None;
    let mut last_error = None;
    for _ in 0..samples.max(1) {
        match exchange(address, server, timeout) {
            Ok(m) => {
                if best.as_ref().map_or(true, |b| m.round_trip_ms < b.round_trip_ms) {
                    best = Some(m);
                }
            }
            Err(error) => {
                last_error = Some(error);
                // Do not keep waiting on a server that has stopped answering.
                if best.is_none() {
                    break;
                }
            }
        }
    }
    best.ok_or_else(|| last_error.unwrap_or(Error::NoServers))
}

/// Tries each server in order and returns the first successful measurement.
pub fn measure(servers: &[&str], timeout: Duration, samples: usize) -> Result<Measurement, Error> {
    let mut last = Error::NoServers;
    for server in servers {
        match query(server, timeout, samples) {
            Ok(measurement) => return Ok(measurement),
            Err(error) => last = error,
        }
    }
    match last {
        Error::NoServers => Err(Error::NoServers),
        other => Err(Error::AllServersFailed(Box::new(other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    const MS: i64 = 1_000_000;

    #[test]
    fn request_has_version_4_client_header() {
        let packet = encode_request(Timestamp { seconds: 0x0102_0304, fraction: 0x0506_0708 });
        assert_eq!(packet[0], 0x23);
        assert!(packet[1..40].iter().all(|&byte| byte == 0));
        assert_eq!(&packet[40..48], &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn timestamp_round_trips_within_a_nanosecond() {
        // 2026-11-19T00:00:00.123456789Z
        let nanos = 1_795_046_400 * NANOS_PER_SEC + 123_456_789;
        let back = Timestamp::from_unix_nanos(nanos).to_unix_nanos();
        assert!((back - nanos).abs() <= 1, "difference {} ns", back - nanos);
    }

    #[test]
    fn unix_epoch_maps_to_the_known_ntp_second() {
        let ts = Timestamp::from_unix_nanos(0);
        assert_eq!(ts, Timestamp { seconds: 2_208_988_800, fraction: 0 });
        assert_eq!(ts.to_unix_nanos(), 0);
    }

    #[test]
    fn era_rollover_in_2036_is_handled() {
        // 2036-02-07T06:28:16Z is the first second of NTP era 1.
        let rollover = 2_085_978_496 * NANOS_PER_SEC;
        let ts = Timestamp::from_unix_nanos(rollover);
        assert_eq!(ts.seconds, 0);
        assert_eq!(ts.to_unix_nanos(), rollover);
        let before = Timestamp::from_unix_nanos(rollover - NANOS_PER_SEC);
        assert_eq!(before.seconds, u32::MAX);
        assert_eq!(before.to_unix_nanos(), rollover - NANOS_PER_SEC);
    }

    #[test]
    fn offset_and_delay_follow_the_rfc_formula() {
        // Local clock 250 ms slow; 40 ms out, 5 ms in the server, 60 ms back.
        let t1 = 1_000 * MS;
        let t2 = t1 + 250 * MS + 40 * MS;
        let t3 = t2 + 5 * MS;
        let t4 = t1 + 105 * MS;
        let (offset, delay) = offset_and_delay(t1, t2, t3, t4);
        // The asymmetric path (40 out, 60 back) biases the estimate by 10 ms.
        assert_eq!(offset, 240 * MS);
        assert_eq!(delay, 100 * MS);
    }

    #[test]
    fn symmetric_path_gives_the_exact_offset() {
        let t1 = 5_000 * MS;
        let t2 = t1 - 1_500 * MS + 20 * MS;
        let t3 = t2 + MS;
        let t4 = t1 + 41 * MS;
        assert_eq!(offset_and_delay(t1, t2, t3, t4), (-1_500 * MS, 40 * MS));
    }

    fn reply_for(sent: Timestamp) -> [u8; PACKET_LEN] {
        let mut packet = [0u8; PACKET_LEN];
        packet[0] = (4 << 3) | 4;
        packet[1] = 2;
        sent.write(&mut packet[24..32]);
        Timestamp { seconds: 3_000_000_000, fraction: 1 }.write(&mut packet[32..40]);
        Timestamp { seconds: 3_000_000_000, fraction: 2 }.write(&mut packet[40..48]);
        packet
    }

    #[test]
    fn valid_reply_is_decoded() {
        let sent = Timestamp { seconds: 42, fraction: 7 };
        let reply = decode_reply(&reply_for(sent), sent).expect("valid reply");
        assert_eq!(reply.stratum, 2);
        assert_eq!(reply.receive.fraction, 1);
        assert_eq!(reply.transmit.fraction, 2);
    }

    #[test]
    fn invalid_replies_are_rejected() {
        let sent = Timestamp { seconds: 42, fraction: 7 };
        let good = reply_for(sent);

        assert!(matches!(decode_reply(&good[..47], sent), Err(Error::Truncated(47))));

        let mut client_mode = good;
        client_mode[0] = (4 << 3) | 3;
        assert!(matches!(decode_reply(&client_mode, sent), Err(Error::UnexpectedMode(3))));

        let mut kiss = good;
        kiss[1] = 0;
        kiss[12..16].copy_from_slice(b"RATE");
        assert!(matches!(decode_reply(&kiss, sent), Err(Error::KissOfDeath(code)) if &code == b"RATE"));

        let mut alarm = good;
        alarm[0] |= 0b1100_0000;
        assert!(matches!(decode_reply(&alarm, sent), Err(Error::Unsynchronised)));

        let mut no_time = good;
        no_time[40..48].fill(0);
        assert!(matches!(decode_reply(&no_time, sent), Err(Error::Unsynchronised)));

        let other = Timestamp { seconds: 43, fraction: 7 };
        assert!(matches!(decode_reply(&good, other), Err(Error::OriginMismatch)));
    }

    /// Runs a one-shot NTP server on the loopback interface whose clock is
    /// `skew` nanoseconds ahead of the system clock.
    fn spawn_server(skew: i64, replies: usize) -> SocketAddr {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("bind loopback");
        let address = socket.local_addr().expect("local address");
        thread::spawn(move || {
            let mut buffer = [0u8; PACKET_LEN];
            for _ in 0..replies {
                let Ok((len, peer)) = socket.recv_from(&mut buffer) else { return };
                if len < PACKET_LEN {
                    continue;
                }
                let now = Timestamp::from_unix_nanos(system_unix_nanos() + skew);
                let mut reply = [0u8; PACKET_LEN];
                reply[0] = (4 << 3) | 4;
                reply[1] = 1;
                reply[24..32].copy_from_slice(&buffer[40..48]);
                now.write(&mut reply[32..40]);
                now.write(&mut reply[40..48]);
                let _ = socket.send_to(&reply, peer);
            }
        });
        address
    }

    #[test]
    fn measures_a_known_skew_over_loopback() {
        let skew = 3_210 * MS;
        let server = spawn_server(skew, 3).to_string();
        let m = query(&server, Duration::from_secs(2), 3).expect("loopback query");
        assert!((m.offset_ms - 3_210.0).abs() < 50.0, "offset was {} ms", m.offset_ms);
        assert!(m.round_trip_ms < 500.0);
        assert_eq!(m.stratum, 1);
        assert_eq!(m.server, server);
    }

    #[test]
    fn falls_through_to_the_next_server() {
        // A bound socket that never answers stands in for a dead server.
        let dead = UdpSocket::bind("127.0.0.1:0").expect("bind loopback");
        let dead_address = dead.local_addr().expect("local address").to_string();
        let live = spawn_server(-750 * MS, 1).to_string();

        let servers = [dead_address.as_str(), live.as_str()];
        let m = measure(&servers, Duration::from_millis(200), 1).expect("second server answers");
        assert_eq!(m.server, live);
        assert!((m.offset_ms + 750.0).abs() < 50.0, "offset was {} ms", m.offset_ms);
    }

    #[test]
    fn reports_failure_when_nothing_answers() {
        let dead = UdpSocket::bind("127.0.0.1:0").expect("bind loopback");
        let address = dead.local_addr().expect("local address").to_string();
        let error = measure(&[address.as_str()], Duration::from_millis(100), 1).unwrap_err();
        assert!(matches!(error, Error::AllServersFailed(_)));
        assert!(matches!(measure(&[], Duration::from_millis(100), 1), Err(Error::NoServers)));
    }
}
