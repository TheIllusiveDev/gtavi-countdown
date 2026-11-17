using GtaviCountdown.UI.Platform;

namespace GtaviCountdown.UI;

/// <summary>Where the network clock correction stands.</summary>
public abstract record SyncState
{
    private SyncState() { }

    public sealed record Idle : SyncState;

    public sealed record Syncing : SyncState;

    /// <param name="At">When the measurement was taken, in Unix milliseconds.</param>
    public sealed record Ok(TimeSyncResult Result, long At) : SyncState;

    public sealed record Error(string Message) : SyncState;
}
