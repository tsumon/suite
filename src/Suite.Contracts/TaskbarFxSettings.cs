namespace Suite.Contracts;

/// <summary>
/// Taskbar appearance preferences only. No pixels, no secrets.
/// Default keep-on-exit matches ADR: avoid flashing the taskbar when Suite closes.
/// </summary>
public sealed class TaskbarFxSettings
{
    /// <summary>Default dark translucent ARGB used for opaque/acrylic until the user picks a color.</summary>
    public const uint DefaultArgb = 0xCC1F1F1F;

    public bool Enabled { get; set; }

    /// <summary>Wire value: normal | opaque | clear | acrylic.</summary>
    public string Mode { get; set; } = TaskbarAppearanceModes.Acrylic;

    /// <summary>AARRGGBB. Alpha is used for acrylic/clear; opaque forces 0xFF.</summary>
    public uint Argb { get; set; } = DefaultArgb;

    /// <summary>
    /// When true (default), closing Suite does not Shutdown TaskbarFx, so the last
    /// appearance stays. Turning the feature off still Disable + Shutdown.
    /// </summary>
    public bool KeepAppearanceIfSuiteExits { get; set; } = true;

    public TaskbarAppearanceMode ParsedMode =>
        TaskbarAppearanceModes.TryParse(Mode, out TaskbarAppearanceMode mode)
            ? mode
            : TaskbarAppearanceMode.Acrylic;

    public TaskbarFxSettings Clone() => new()
    {
        Enabled = Enabled,
        Mode = Mode,
        Argb = Argb,
        KeepAppearanceIfSuiteExits = KeepAppearanceIfSuiteExits,
    };
}
