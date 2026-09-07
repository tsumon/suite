using System.Text.Json;
using System.Text.Json.Serialization;

namespace Suite.Contracts;

public static class SettingsJson
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true,
        WriteIndented = true,
        AllowTrailingCommas = true,
        ReadCommentHandling = JsonCommentHandling.Skip,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public static string Serialize(AppSettings settings) =>
        JsonSerializer.Serialize(settings, Options);

    public static AppSettings Deserialize(string json)
    {
        var settings = JsonSerializer.Deserialize<AppSettings>(json, Options) ?? AppSettings.CreateDefault();
        settings.Hotkey ??= HotkeyBinding.Default.Clone();
        settings.NetSpeed ??= new NetSpeedSettings();
        settings.NetSpeed.Normalize();
        settings.Capture ??= new CaptureSettings();
        settings.Capture.SaveDirectory ??= "";
        settings.Capture.Normalize();
        settings.TaskbarFx ??= new TaskbarFxSettings();
        settings.Update ??= new UpdateSettings();
        settings.Update.Normalize();
        settings.Advanced ??= new AdvancedSettings();
        if (string.IsNullOrWhiteSpace(settings.TaskbarFx.Mode)
            || !TaskbarAppearanceModes.TryParse(settings.TaskbarFx.Mode, out _))
        {
            settings.TaskbarFx.Mode = TaskbarAppearanceModes.Acrylic;
        }
        if (settings.Hotkey.VirtualKey is <= 0 or > 0xFE)
        {
            settings.Hotkey.VirtualKey = HotkeyBinding.DefaultVirtualKey;
        }

        if (settings.SchemaVersion < 2)
        {
            settings.NetSpeed.EmbedInTaskbar = true;
            settings.SchemaVersion = 2;
        }

        // Joe: F1 screenshot; after region show toolbar (not auto-pin).
        if (settings.SchemaVersion < 3)
        {
            settings.Hotkey = new HotkeyBinding
            {
                Control = false,
                Shift = false,
                Alt = false,
                Win = false,
                VirtualKey = HotkeyBinding.DefaultVirtualKey,
            };
            settings.Capture.PinAfterCapture = false;
            settings.SchemaVersion = 3;
        }

        // P2: fill nested defaults without rewriting user choices already present.
        if (settings.SchemaVersion < 4)
        {
            settings.Update ??= new UpdateSettings();
            settings.Advanced ??= new AdvancedSettings();
            // Schema <4: ShowMagnifier/HistoryEnabled defaults come from property initializers.
            if (settings.Capture.HistoryMax <= 0)
            {
                settings.Capture.HistoryMax = 30;
            }

            settings.Capture.ColorPickHotkey ??= HotkeyBinding.DefaultColorPick.Clone();
            settings.Capture.PinClickThroughHotkey ??= HotkeyBinding.DefaultPinClickThrough.Clone();
            // EmbedSecondary stays false (P2 default off).
            settings.SchemaVersion = AppSettings.CurrentSchemaVersion;
        }

        if (settings.SchemaVersion <= 0)
        {
            settings.SchemaVersion = AppSettings.CurrentSchemaVersion;
        }

        settings.Capture.Normalize();
        settings.Update.Normalize();
        return settings;
    }
}
