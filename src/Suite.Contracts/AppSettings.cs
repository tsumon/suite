namespace Suite.Contracts;

/// <summary>
/// Persisted preferences only. No screenshot pixels, no secrets, no keys.
/// Written solely by Suite.exe to %LOCALAPPDATA%\Suite\settings.json.
/// </summary>
public sealed class AppSettings
{
    public const int CurrentSchemaVersion = 4;

    public int SchemaVersion { get; set; } = CurrentSchemaVersion;
    public bool StartWithWindows { get; set; }
    public HotkeyBinding Hotkey { get; set; } = HotkeyBinding.Default.Clone();
    public NetSpeedSettings NetSpeed { get; set; } = new();
    public CaptureSettings Capture { get; set; } = new();
    public TaskbarFxSettings TaskbarFx { get; set; } = new();
    public UpdateSettings Update { get; set; } = new();
    public AdvancedSettings Advanced { get; set; } = new();

    public static AppSettings CreateDefault() => new();

    public AppSettings Clone() => new()
    {
        SchemaVersion = SchemaVersion,
        StartWithWindows = StartWithWindows,
        Hotkey = Hotkey.Clone(),
        NetSpeed = NetSpeed.Clone(),
        Capture = Capture.Clone(),
        TaskbarFx = TaskbarFx.Clone(),
        Update = Update.Clone(),
        Advanced = Advanced.Clone(),
    };
}
