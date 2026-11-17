using System.Buffers.Binary;
using System.Diagnostics;
using System.Net;
using System.Net.Sockets;
using System.Runtime.Versioning;

namespace GtaviCountdown.Core.TimeSync;

/// <summary>
/// A small SNTP client (RFC 4330) built on the base class library only.
/// </summary>
/// <remarks>
/// It answers one question: by how much is the system clock wrong? It sends a
/// client-mode NTP packet, validates the reply and applies the standard
/// four-timestamp calculation:
/// <code>
/// offset = ((t2 - t1) + (t3 - t4)) / 2
/// delay  =  (t4 - t1) - (t3 - t2)
/// </code>
/// where t1 and t4 are the local send and receive times and t2 and t3 are the
/// server's receive and transmit times. The elapsed time between t1 and t4 is
/// taken from a monotonic clock, so a clock step during the exchange cannot
/// corrupt the delay.
/// </remarks>
public static class Sntp
{
    /// <summary>Size of an NTP packet without extension fields or authentication.</summary>
    public const int PacketLength = 48;

    /// <summary>Seconds between the NTP epoch (1900-01-01) and the Unix epoch (1970-01-01).</summary>
    internal const long NtpUnixOffsetSeconds = 2_208_988_800;

    internal const long NanosPerSecond = 1_000_000_000;

    /// <summary>Builds a client request carrying <paramref name="transmit"/> as its transmit timestamp.</summary>
    public static byte[] EncodeRequest(NtpTimestamp transmit)
    {
        var packet = new byte[PacketLength];
        // Leap indicator 0 (no warning), version 4, mode 3 (client).
        packet[0] = (4 << 3) | 3;
        transmit.Write(packet.AsSpan(40, 8));
        return packet;
    }

    /// <summary>Parses and validates a reply to a request sent with timestamp <paramref name="sent"/>.</summary>
    /// <exception cref="SntpException">The reply is unusable.</exception>
    public static NtpReply DecodeReply(ReadOnlySpan<byte> packet, NtpTimestamp sent)
    {
        if (packet.Length < PacketLength)
            throw new SntpException(SntpError.Truncated, $"truncated reply ({packet.Length} bytes)");
        var leap = packet[0] >> 6;
        var mode = packet[0] & 0b111;
        var stratum = packet[1];
        if (mode != 4)
            throw new SntpException(SntpError.UnexpectedMode, $"unexpected packet mode {mode}");
        if (stratum == 0)
        {
            var code = System.Text.Encoding.ASCII.GetString(packet.Slice(12, 4));
            throw new SntpException(SntpError.KissOfDeath, $"the server refused the request ({code})") { KissCode = code };
        }
        var transmit = NtpTimestamp.Read(packet.Slice(40, 8));
        if (leap == 3 || stratum > 15 || (transmit.Seconds == 0 && transmit.Fraction == 0))
            throw new SntpException(SntpError.Unsynchronised, "the server clock is not synchronised");
        if (NtpTimestamp.Read(packet.Slice(24, 8)) != sent)
            throw new SntpException(SntpError.OriginMismatch, "the reply does not match the request");
        return new NtpReply(stratum, NtpTimestamp.Read(packet.Slice(32, 8)), transmit);
    }

    /// <summary>
    /// Applies the SNTP on-wire calculation. All arguments are nanoseconds since
    /// the Unix epoch; the result is <c>(offset, delay)</c> in nanoseconds.
    /// </summary>
    public static (long Offset, long Delay) OffsetAndDelay(long t1, long t2, long t3, long t4)
    {
        var offset = ((t2 - t1) + (t3 - t4)) / 2;
        var delay = (t4 - t1) - (t3 - t2);
        return (offset, Math.Max(delay, 0));
    }

    internal static long SystemUnixNanos() =>
        (DateTime.UtcNow.Ticks - DateTime.UnixEpoch.Ticks) * 100;

    /// <summary>Splits <c>host:port</c>, accepting bracketed IPv6 literals.</summary>
    private static (string Host, int Port) SplitHostPort(string server)
    {
        var cut = server.LastIndexOf(':');
        if (cut <= 0 || !int.TryParse(server.AsSpan(cut + 1), out var port) || port is < 1 or > 65535)
            throw new SntpException(SntpError.Resolve, $"could not resolve {server}");
        var host = server[..cut].Trim('[', ']');
        return (host, port);
    }

    [UnsupportedOSPlatform("browser")]
    private static IPEndPoint ResolveServer(string server)
    {
        var (host, port) = SplitHostPort(server);
        if (IPAddress.TryParse(host, out var literal)) return new IPEndPoint(literal, port);
        try
        {
            var address = Dns.GetHostAddresses(host).FirstOrDefault()
                ?? throw new SntpException(SntpError.Resolve, $"could not resolve {server}");
            return new IPEndPoint(address, port);
        }
        catch (SocketException)
        {
            throw new SntpException(SntpError.Resolve, $"could not resolve {server}");
        }
    }

    /// <summary>Performs one exchange with the server at <paramref name="address"/>.</summary>
    [UnsupportedOSPlatform("browser")]
    private static SntpMeasurement Exchange(IPEndPoint address, string server, TimeSpan timeout)
    {
        try
        {
            using var socket = new Socket(address.AddressFamily, SocketType.Dgram, ProtocolType.Udp);
            socket.ReceiveTimeout = (int)Math.Max(1, timeout.TotalMilliseconds);
            socket.SendTimeout = (int)Math.Max(1, timeout.TotalMilliseconds);
            // Connecting filters out datagrams from any other source address.
            socket.Connect(address);

            var t1 = SystemUnixNanos();
            var started = Stopwatch.GetTimestamp();
            // The request carries the truncated NTP form of t1; the server echoes it.
            var sent = NtpTimestamp.FromUnixNanos(t1);
            socket.Send(EncodeRequest(sent));

            var buffer = new byte[512];
            while (true)
            {
                var remaining = timeout - Stopwatch.GetElapsedTime(started);
                if (remaining <= TimeSpan.Zero)
                    throw new SntpException(SntpError.Timeout, "the time server did not answer");
                socket.ReceiveTimeout = (int)Math.Max(1, remaining.TotalMilliseconds);
                var received = socket.Receive(buffer);
                var elapsed = Stopwatch.GetElapsedTime(started);
                NtpReply reply;
                try
                {
                    reply = DecodeReply(buffer.AsSpan(0, received), sent);
                }
                catch (SntpException e) when (e.Error == SntpError.OriginMismatch)
                {
                    // A stray or delayed datagram: keep waiting for the real
                    // reply until the deadline passes.
                    continue;
                }
                var t4 = t1 + elapsed.Ticks * 100;
                var (offset, delay) = OffsetAndDelay(t1, reply.Receive.ToUnixNanos(), reply.Transmit.ToUnixNanos(), t4);
                return new SntpMeasurement(offset / 1e6, delay / 1e6, reply.Stratum, server);
            }
        }
        catch (SocketException e) when (e.SocketErrorCode is SocketError.TimedOut or SocketError.WouldBlock)
        {
            throw new SntpException(SntpError.Timeout, "the time server did not answer", e);
        }
        catch (SocketException e)
        {
            throw new SntpException(SntpError.Network, $"network error: {e.Message}", e);
        }
    }

    /// <summary>
    /// Measures the clock offset against one server, given as <c>host:port</c>.
    /// </summary>
    /// <remarks>
    /// Up to <paramref name="samples"/> exchanges are made and the one with the
    /// shortest round trip is returned: the less time a packet spends in
    /// transit, the smaller the possible asymmetry between its two legs, and
    /// asymmetry is the error.
    /// </remarks>
    [UnsupportedOSPlatform("browser")]
    public static SntpMeasurement Query(string server, TimeSpan timeout, int samples)
    {
        var address = ResolveServer(server);
        SntpMeasurement? best = null;
        SntpException? lastError = null;
        for (var i = 0; i < Math.Max(1, samples); i++)
        {
            try
            {
                var m = Exchange(address, server, timeout);
                if (best is null || m.RoundTripMs < best.RoundTripMs) best = m;
            }
            catch (SntpException e)
            {
                lastError = e;
                // Do not keep waiting on a server that has stopped answering.
                if (best is null) break;
            }
        }
        return best ?? throw lastError ?? new SntpException(SntpError.NoServers, "no time servers are configured");
    }

    /// <summary>Tries each server in order and returns the first successful measurement.</summary>
    [UnsupportedOSPlatform("browser")]
    public static SntpMeasurement Measure(IReadOnlyList<string> servers, TimeSpan timeout, int samples)
    {
        SntpException? last = null;
        foreach (var server in servers)
        {
            try
            {
                return Query(server, timeout, samples);
            }
            catch (SntpException e)
            {
                last = e;
            }
        }
        if (last is null) throw new SntpException(SntpError.NoServers, "no time servers are configured");
        throw new SntpException(SntpError.AllServersFailed, $"no time server could be reached: {last.Message}", last);
    }
}

/// <summary>A 64-bit NTP timestamp: whole seconds and a binary fraction of a second.</summary>
public readonly record struct NtpTimestamp(uint Seconds, uint Fraction)
{
    /// <summary>
    /// Converts nanoseconds since the Unix epoch to an NTP timestamp. The seconds
    /// field wraps every 2^32 seconds, as the protocol defines.
    /// </summary>
    public static NtpTimestamp FromUnixNanos(long nanos)
    {
        var secs = Math.DivRem(nanos, Sntp.NanosPerSecond, out var sub);
        if (sub < 0)
        {
            secs -= 1;
            sub += Sntp.NanosPerSecond;
        }
        secs += Sntp.NtpUnixOffsetSeconds;
        return new NtpTimestamp(unchecked((uint)secs), (uint)(((ulong)sub << 32) / (ulong)Sntp.NanosPerSecond));
    }

    /// <summary>
    /// Converts to nanoseconds since the Unix epoch.
    /// </summary>
    /// <remarks>
    /// The 32-bit seconds field is ambiguous across eras. Values with the top bit
    /// clear are taken to lie in era 1 (from 7 February 2036), which keeps the
    /// conversion correct from 1968 until 2104.
    /// </remarks>
    public long ToUnixNanos()
    {
        long secs = Seconds;
        if ((Seconds & 0x8000_0000) == 0) secs += 1L << 32;
        var nanos = ((ulong)Fraction * (ulong)Sntp.NanosPerSecond) >> 32;
        return (secs - Sntp.NtpUnixOffsetSeconds) * Sntp.NanosPerSecond + (long)nanos;
    }

    internal static NtpTimestamp Read(ReadOnlySpan<byte> bytes) =>
        new(BinaryPrimitives.ReadUInt32BigEndian(bytes), BinaryPrimitives.ReadUInt32BigEndian(bytes[4..]));

    public void Write(Span<byte> bytes)
    {
        BinaryPrimitives.WriteUInt32BigEndian(bytes, Seconds);
        BinaryPrimitives.WriteUInt32BigEndian(bytes[4..], Fraction);
    }
}

/// <summary>The fields of a validated server reply that the calculation needs.</summary>
/// <param name="Receive">When the request arrived at the server (t2).</param>
/// <param name="Transmit">When the reply left the server (t3).</param>
public readonly record struct NtpReply(byte Stratum, NtpTimestamp Receive, NtpTimestamp Transmit);

/// <summary>The result of one successful exchange with a time server.</summary>
/// <param name="OffsetMs">Milliseconds to add to the system clock to obtain server time.</param>
/// <param name="RoundTripMs">Round-trip network delay in milliseconds, excluding server processing.</param>
/// <param name="Stratum">Stratum of the server: 1 is a primary reference, 2 to 15 are secondary.</param>
/// <param name="Server">The host the measurement was taken from, as passed by the caller.</param>
public sealed record SntpMeasurement(double OffsetMs, double RoundTripMs, byte Stratum, string Server);

public enum SntpError
{
    /// <summary>The host name did not resolve to any address.</summary>
    Resolve,
    /// <summary>A socket operation failed.</summary>
    Network,
    /// <summary>The server did not answer in time.</summary>
    Timeout,
    /// <summary>The reply was shorter than an NTP header.</summary>
    Truncated,
    /// <summary>The reply was not a server-mode packet.</summary>
    UnexpectedMode,
    /// <summary>The server declined to serve time (stratum 0, "kiss-o'-death").</summary>
    KissOfDeath,
    /// <summary>The server is not synchronised to a reference clock.</summary>
    Unsynchronised,
    /// <summary>The reply does not echo the timestamp that was sent; it may be forged or belong to an earlier request.</summary>
    OriginMismatch,
    /// <summary>Every configured server failed. The inner exception holds the last failure.</summary>
    AllServersFailed,
    /// <summary>No servers were configured.</summary>
    NoServers,
}

/// <summary>Why a measurement could not be taken. Messages are lower-case fragments, like the Rust original.</summary>
public sealed class SntpException(SntpError error, string message, Exception? inner = null) : Exception(message, inner)
{
    public SntpError Error { get; } = error;

    /// <summary>The four-character code of a kiss-o'-death reply.</summary>
    public string? KissCode { get; init; }
}
