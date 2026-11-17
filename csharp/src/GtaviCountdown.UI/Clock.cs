using GtaviCountdown.Core;

namespace GtaviCountdown.UI;

/// <summary>
/// A wall clock that ticks once per second, in phase with a target.
/// </summary>
/// <remarks>
/// <see cref="Now"/> only changes when the displayed seconds value changes, so
/// the UI re-renders at most once per second. Sub-second output (the optional
/// milliseconds readout) is painted by the browser from an animation frame.
///
/// Every tick re-reads the system clock, so a throttled background tab, a
/// suspended laptop or a late timer can delay an update but never skew it.
/// </remarks>
public sealed class Clock : IDisposable
{
    private readonly Lock _gate = new();
    private readonly Timer _timer;
    private long _target;
    private bool _running;

    public Clock()
    {
        _timer = new Timer(_ => Resync(), null, Timeout.Infinite, Timeout.Infinite);
        Now = Precise();
    }

    /// <summary>Raised after every tick, on whatever thread the timer fired on.</summary>
    public event Action? Ticked;

    /// <summary>Corrected current time in Unix milliseconds, updated once per second.</summary>
    public long Now { get; private set; }

    /// <summary>
    /// Correction added to the system clock, in milliseconds. Set from a
    /// network time measurement on the desktop; zero when the system clock is trusted.
    /// </summary>
    public double Offset { get; private set; }

    /// <summary>Corrected current time, read directly from the system clock.</summary>
    public long Precise() => DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() + (long)Math.Round(Offset);

    /// <summary>Starts ticking in phase with <paramref name="target"/>.</summary>
    public void Start(long target)
    {
        lock (_gate)
        {
            _target = target;
            _running = true;
        }
        Resync();
    }

    /// <summary>Re-aligns the tick with a new target, for example after a zone change.</summary>
    public void Retarget(long target)
    {
        lock (_gate) _target = target;
        Resync();
    }

    /// <summary>Applies a new clock correction and re-aligns the tick immediately.</summary>
    public void SetOffset(double offset)
    {
        lock (_gate) Offset = offset;
        Resync();
    }

    /// <summary>Reads the clock now and schedules the next tick on the next change of the display.</summary>
    public void Resync()
    {
        lock (_gate)
        {
            if (!_running) return;
            Now = Precise();
            // Timers may fire a fraction of a millisecond early. Should that
            // happen, this simply runs again after a millisecond or two.
            _timer.Change(Countdown.MsUntilNextSecond(Now, _target), Timeout.Infinite);
        }
        Ticked?.Invoke();
    }

    public void Stop()
    {
        lock (_gate)
        {
            _running = false;
            _timer.Change(Timeout.Infinite, Timeout.Infinite);
        }
    }

    public void Dispose()
    {
        Stop();
        _timer.Dispose();
    }
}
