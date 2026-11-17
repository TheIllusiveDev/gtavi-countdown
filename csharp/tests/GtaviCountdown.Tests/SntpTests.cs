using System.Net;
using System.Net.Sockets;
using GtaviCountdown.Core.TimeSync;

namespace GtaviCountdown.Tests;

public class SntpTests
{
    private const long Ms = 1_000_000;
    private const long NanosPerSecond = 1_000_000_000;

    [Fact]
    public void RequestHasVersion4ClientHeader()
    {
        var packet = Sntp.EncodeRequest(new NtpTimestamp(0x0102_0304, 0x0506_0708));
        Assert.Equal(0x23, packet[0]);
        Assert.All(packet[1..40], b => Assert.Equal(0, b));
        Assert.Equal(new byte[] { 1, 2, 3, 4, 5, 6, 7, 8 }, packet[40..48]);
    }

    [Fact]
    public void TimestampRoundTripsWithinANanosecond()
    {
        // 2026-11-19T00:00:00.123456789Z
        var nanos = 1_795_046_400 * NanosPerSecond + 123_456_789;
        var back = NtpTimestamp.FromUnixNanos(nanos).ToUnixNanos();
        Assert.InRange(back - nanos, -1, 1);
    }

    [Fact]
    public void UnixEpochMapsToTheKnownNtpSecond()
    {
        var ts = NtpTimestamp.FromUnixNanos(0);
        Assert.Equal(new NtpTimestamp(2_208_988_800, 0), ts);
        Assert.Equal(0, ts.ToUnixNanos());
    }

    [Fact]
    public void EraRolloverIn2036IsHandled()
    {
        // 2036-02-07T06:28:16Z is the first second of NTP era 1.
        var rollover = 2_085_978_496 * NanosPerSecond;
        var ts = NtpTimestamp.FromUnixNanos(rollover);
        Assert.Equal(0u, ts.Seconds);
        Assert.Equal(rollover, ts.ToUnixNanos());
        var before = NtpTimestamp.FromUnixNanos(rollover - NanosPerSecond);
        Assert.Equal(uint.MaxValue, before.Seconds);
        Assert.Equal(rollover - NanosPerSecond, before.ToUnixNanos());
    }

    [Fact]
    public void NegativeInstantsRoundDownToTheEarlierSecond()
    {
        var ts = NtpTimestamp.FromUnixNanos(-1);
        Assert.Equal(2_208_988_799u, ts.Seconds);
        Assert.InRange(ts.ToUnixNanos(), -2, 0);
    }

    [Fact]
    public void OffsetAndDelayFollowTheRfcFormula()
    {
        // Local clock 250 ms slow; 40 ms out, 5 ms in the server, 60 ms back.
        var t1 = 1_000 * Ms;
        var t2 = t1 + 250 * Ms + 40 * Ms;
        var t3 = t2 + 5 * Ms;
        var t4 = t1 + 105 * Ms;
        var (offset, delay) = Sntp.OffsetAndDelay(t1, t2, t3, t4);
        // The asymmetric path (40 out, 60 back) biases the estimate by 10 ms.
        Assert.Equal(240 * Ms, offset);
        Assert.Equal(100 * Ms, delay);
    }

    [Fact]
    public void SymmetricPathGivesTheExactOffset()
    {
        var t1 = 5_000 * Ms;
        var t2 = t1 - 1_500 * Ms + 20 * Ms;
        var t3 = t2 + Ms;
        var t4 = t1 + 41 * Ms;
        Assert.Equal((-1_500 * Ms, 40 * Ms), Sntp.OffsetAndDelay(t1, t2, t3, t4));
    }

    private static byte[] ReplyFor(NtpTimestamp sent)
    {
        var packet = new byte[Sntp.PacketLength];
        packet[0] = (4 << 3) | 4;
        packet[1] = 2;
        sent.Write(packet.AsSpan(24, 8));
        new NtpTimestamp(3_000_000_000, 1).Write(packet.AsSpan(32, 8));
        new NtpTimestamp(3_000_000_000, 2).Write(packet.AsSpan(40, 8));
        return packet;
    }

    [Fact]
    public void ValidReplyIsDecoded()
    {
        var sent = new NtpTimestamp(42, 7);
        var reply = Sntp.DecodeReply(ReplyFor(sent), sent);
        Assert.Equal(2, reply.Stratum);
        Assert.Equal(1u, reply.Receive.Fraction);
        Assert.Equal(2u, reply.Transmit.Fraction);
    }

    private static SntpError ErrorOf(byte[] packet, NtpTimestamp sent) =>
        Assert.Throws<SntpException>(() => Sntp.DecodeReply(packet, sent)).Error;

    [Fact]
    public void InvalidRepliesAreRejected()
    {
        var sent = new NtpTimestamp(42, 7);
        var good = ReplyFor(sent);

        Assert.Equal(SntpError.Truncated, ErrorOf(good[..47], sent));

        var clientMode = (byte[])good.Clone();
        clientMode[0] = (4 << 3) | 3;
        Assert.Equal(SntpError.UnexpectedMode, ErrorOf(clientMode, sent));

        var kiss = (byte[])good.Clone();
        kiss[1] = 0;
        "RATE"u8.CopyTo(kiss.AsSpan(12));
        var kissError = Assert.Throws<SntpException>(() => Sntp.DecodeReply(kiss, sent));
        Assert.Equal(SntpError.KissOfDeath, kissError.Error);
        Assert.Equal("RATE", kissError.KissCode);

        var alarm = (byte[])good.Clone();
        alarm[0] |= 0b1100_0000;
        Assert.Equal(SntpError.Unsynchronised, ErrorOf(alarm, sent));

        var noTime = (byte[])good.Clone();
        Array.Clear(noTime, 40, 8);
        Assert.Equal(SntpError.Unsynchronised, ErrorOf(noTime, sent));

        Assert.Equal(SntpError.OriginMismatch, ErrorOf(good, new NtpTimestamp(43, 7)));
    }

    /// <summary>
    /// Runs an NTP server on the loopback interface whose clock is
    /// <paramref name="skew"/> nanoseconds ahead of the system clock.
    /// </summary>
    private static string SpawnServer(long skew, int replies)
    {
        var socket = new UdpClient(new IPEndPoint(IPAddress.Loopback, 0));
        var address = (IPEndPoint)socket.Client.LocalEndPoint!;
        _ = Task.Run(async () =>
        {
            using var _ = socket;
            for (var i = 0; i < replies; i++)
            {
                var request = await socket.ReceiveAsync();
                if (request.Buffer.Length < Sntp.PacketLength) continue;
                var now = NtpTimestamp.FromUnixNanos(Sntp.SystemUnixNanos() + skew);
                var reply = new byte[Sntp.PacketLength];
                reply[0] = (4 << 3) | 4;
                reply[1] = 1;
                request.Buffer.AsSpan(40, 8).CopyTo(reply.AsSpan(24));
                now.Write(reply.AsSpan(32, 8));
                now.Write(reply.AsSpan(40, 8));
                await socket.SendAsync(reply, request.RemoteEndPoint);
            }
        });
        return $"127.0.0.1:{address.Port}";
    }

    /// <summary>A bound socket that never answers stands in for a dead server.</summary>
    private static (UdpClient Socket, string Address) DeadServer()
    {
        var socket = new UdpClient(new IPEndPoint(IPAddress.Loopback, 0));
        return (socket, $"127.0.0.1:{((IPEndPoint)socket.Client.LocalEndPoint!).Port}");
    }

    [Fact]
    public void MeasuresAKnownSkewOverLoopback()
    {
        var server = SpawnServer(3_210 * Ms, 3);
        var m = Sntp.Query(server, TimeSpan.FromSeconds(2), 3);
        Assert.InRange(m.OffsetMs, 3_210 - 50, 3_210 + 50);
        Assert.True(m.RoundTripMs < 500);
        Assert.Equal(1, m.Stratum);
        Assert.Equal(server, m.Server);
    }

    [Fact]
    public void FallsThroughToTheNextServer()
    {
        var (dead, deadAddress) = DeadServer();
        using var _ = dead;
        var live = SpawnServer(-750 * Ms, 1);

        var m = Sntp.Measure([deadAddress, live], TimeSpan.FromMilliseconds(200), 1);
        Assert.Equal(live, m.Server);
        Assert.InRange(m.OffsetMs, -750 - 50, -750 + 50);
    }

    [Fact]
    public void ReportsFailureWhenNothingAnswers()
    {
        var (dead, address) = DeadServer();
        using var _ = dead;
        var error = Assert.Throws<SntpException>(() => Sntp.Measure([address], TimeSpan.FromMilliseconds(100), 1));
        Assert.Equal(SntpError.AllServersFailed, error.Error);
        Assert.Equal(SntpError.Timeout, ((SntpException)error.InnerException!).Error);
        Assert.Equal("no time server could be reached: the time server did not answer", error.Message);

        var none = Assert.Throws<SntpException>(() => Sntp.Measure([], TimeSpan.FromMilliseconds(100), 1));
        Assert.Equal(SntpError.NoServers, none.Error);
    }

    [Fact]
    public void ReportsUnresolvableServers()
    {
        var error = Assert.Throws<SntpException>(() => Sntp.Query("no-port-here", TimeSpan.FromMilliseconds(100), 1));
        Assert.Equal(SntpError.Resolve, error.Error);
    }
}
