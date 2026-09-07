namespace Suite.Contracts;

/// <summary>
/// Dirty-shutdown marker under LocalAppData\Suite\crash-guard.json.
/// Not user settings — written by host heartbeat / clean exit.
/// </summary>
public sealed class CrashGuardState
{
    public bool CleanExit { get; set; } = true;
    public bool TaskbarFxWasEnabled { get; set; }
    public bool NetSpeedEmbedWasOn { get; set; }
    public long LastStartUnixMs { get; set; }
    public int RecentCrashCount { get; set; }
    public string? LastDegrade { get; set; }
}
