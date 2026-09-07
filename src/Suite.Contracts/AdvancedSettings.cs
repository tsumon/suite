namespace Suite.Contracts;

/// <summary>Logging + crash-recovery related prefs. INTERACTION-P2 §2 / §11.</summary>
public sealed class AdvancedSettings
{
    /// <summary>Write debug log under LocalAppData\Suite\logs. Default off.</summary>
    public bool LoggingEnabled { get; set; }

    /// <summary>
    /// When true (default), abnormal exit while TaskbarFx was on auto-disables FX next launch.
    /// Process-split TaskbarFx remains a future option — not required for P2 fail-safe.
    /// </summary>
    public bool TaskbarFxAutoDisableOnFail { get; set; } = true;

    public AdvancedSettings Clone() => new()
    {
        LoggingEnabled = LoggingEnabled,
        TaskbarFxAutoDisableOnFail = TaskbarFxAutoDisableOnFail,
    };
}
