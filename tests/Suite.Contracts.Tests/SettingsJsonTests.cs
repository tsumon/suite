using System.Text.Json;
using Suite.Contracts;

namespace Suite.Contracts.Tests;

public sealed class SettingsJsonTests
{
    [Fact]
    public void Default_opens_toolbar_embeds_net_speed_and_uses_schema_4()
    {
        var settings = AppSettings.CreateDefault();
        Assert.False(settings.Capture.PinAfterCapture);
        Assert.True(settings.NetSpeed.EmbedInTaskbar);
        Assert.Equal(NetSpeedSettings.DefaultFontSizeDip, settings.NetSpeed.FontSizeDip);
        Assert.True(settings.NetSpeed.ShowDownload);
        Assert.True(settings.NetSpeed.ShowUpload);
        Assert.True(settings.NetSpeed.BackgroundTransparent);
        Assert.False(settings.NetSpeed.ShowBorder);
        Assert.True(settings.NetSpeed.TwoLine);
        Assert.False(settings.NetSpeed.Bold);
        Assert.Null(settings.NetSpeed.DownColorArgb);
        Assert.Null(settings.NetSpeed.UpColorArgb);
        Assert.Equal(4, settings.SchemaVersion);
        Assert.Equal(AppSettings.CurrentSchemaVersion, settings.SchemaVersion);
    }

    [Fact]
    public void Schema_1_migrates_to_taskbar_embed_f1_and_toolbar()
    {
        AppSettings restored = SettingsJson.Deserialize("""{"schemaVersion":1,"capture":{"pinAfterCapture":true},"netSpeed":{"embedInTaskbar":false}}""");
        Assert.False(restored.Capture.PinAfterCapture);
        Assert.True(restored.NetSpeed.EmbedInTaskbar);
        Assert.Equal(4, restored.SchemaVersion);
        Assert.Equal("F1", restored.Hotkey.ToDisplayString());
    }

    [Fact]
    public void Schema_2_migrates_to_f1_and_toolbar_keeps_embed_off()
    {
        AppSettings restored = SettingsJson.Deserialize("""{"schemaVersion":2,"capture":{"pinAfterCapture":true},"netSpeed":{"embedInTaskbar":false}}""");
        Assert.False(restored.Capture.PinAfterCapture);
        Assert.False(restored.NetSpeed.EmbedInTaskbar);
        Assert.Equal(4, restored.SchemaVersion);
        Assert.False(restored.Hotkey.Control);
        Assert.False(restored.Hotkey.Shift);
        Assert.Equal(HotkeyBinding.DefaultVirtualKey, restored.Hotkey.VirtualKey);
    }

    [Fact]
    public void Schema_3_keeps_explicit_pin_and_custom_hotkey()
    {
        AppSettings restored = SettingsJson.Deserialize(
            """{"schemaVersion":3,"capture":{"pinAfterCapture":true},"hotkey":{"control":true,"shift":true,"virtualKey":123}}""");
        Assert.True(restored.Capture.PinAfterCapture);
        Assert.True(restored.Hotkey.Control);
        Assert.True(restored.Hotkey.Shift);
        Assert.Equal(0x7B, restored.Hotkey.VirtualKey);
        Assert.Equal(4, restored.SchemaVersion);
        Assert.True(restored.Capture.ShowMagnifier);
        Assert.True(restored.Capture.HistoryEnabled);
        Assert.False(restored.NetSpeed.EmbedSecondary);
        Assert.Equal(UpdateSettings.ChannelStable, restored.Update.Channel);
    }

    [Fact]
    public void Default_hotkey_is_f1_without_modifiers()
    {
        var settings = AppSettings.CreateDefault();
        Assert.False(settings.Hotkey.Control);
        Assert.False(settings.Hotkey.Shift);
        Assert.False(settings.Hotkey.Alt);
        Assert.False(settings.Hotkey.Win);
        Assert.Equal(0x70, settings.Hotkey.VirtualKey);
        Assert.Equal(HotkeyBinding.DefaultVirtualKey, settings.Hotkey.VirtualKey);
        Assert.Equal("F1", settings.Hotkey.ToDisplayString());
        Assert.Equal(0x72, HotkeyBinding.PinClipboardVirtualKey);
        Assert.Equal("F3", HotkeyBinding.VirtualKeyName(HotkeyBinding.PinClipboardVirtualKey));
    }

    [Fact]
    public void Roundtrip_preserves_start_adapter_hotkey_and_window()
    {
        var original = new AppSettings
        {
            StartWithWindows = true,
            Hotkey = new HotkeyBinding { Control = true, Alt = true, VirtualKey = 0x70 },
            NetSpeed = new NetSpeedSettings
            {
                Visible = false,
                EmbedInTaskbar = false,
                IfIndex = 17,
                AdapterAlias = "Ethernet",
                Left = 120,
                Top = 40,
                FontSizeDip = 15,
                ShowDownload = true,
                ShowUpload = false,
                DownColorArgb = 0xFF00AA00,
                UpColorArgb = 0xFFFF5500,
                BackgroundTransparent = false,
                BackgroundArgb = 0x80202020,
                ShowBorder = true,
                TwoLine = false,
                Bold = true,
            },
            Capture = new CaptureSettings
            {
                SaveFileAfterCapture = true,
                PinAfterCapture = true,
                SaveDirectory = @"C:\Users\Public\SuiteCaptures",
            },
            TaskbarFx = new TaskbarFxSettings
            {
                Enabled = true,
                Mode = TaskbarAppearanceModes.Clear,
                Argb = 0x80FF00AA,
                KeepAppearanceIfSuiteExits = false,
            },
        };

        AppSettings restored = SettingsJson.Deserialize(SettingsJson.Serialize(original));
        Assert.True(restored.StartWithWindows);
        Assert.True(restored.Hotkey.Control);
        Assert.True(restored.Hotkey.Alt);
        Assert.Equal(0x70, restored.Hotkey.VirtualKey);
        Assert.False(restored.NetSpeed.Visible);
        Assert.False(restored.NetSpeed.EmbedInTaskbar);
        Assert.Equal(17u, restored.NetSpeed.IfIndex);
        Assert.Equal("Ethernet", restored.NetSpeed.AdapterAlias);
        Assert.Equal(120, restored.NetSpeed.Left);
        Assert.Equal(40, restored.NetSpeed.Top);
        Assert.Equal(15, restored.NetSpeed.FontSizeDip);
        Assert.True(restored.NetSpeed.ShowDownload);
        Assert.False(restored.NetSpeed.ShowUpload);
        Assert.Equal(0xFF00AA00u, restored.NetSpeed.DownColorArgb);
        Assert.Equal(0xFFFF5500u, restored.NetSpeed.UpColorArgb);
        Assert.False(restored.NetSpeed.BackgroundTransparent);
        Assert.Equal(0x80202020u, restored.NetSpeed.BackgroundArgb);
        Assert.True(restored.NetSpeed.ShowBorder);
        Assert.False(restored.NetSpeed.TwoLine);
        Assert.True(restored.NetSpeed.Bold);
        Assert.True(restored.Capture.SaveFileAfterCapture);
        Assert.True(restored.Capture.PinAfterCapture);
        Assert.Equal(@"C:\Users\Public\SuiteCaptures", restored.Capture.SaveDirectory);
        Assert.True(restored.TaskbarFx.Enabled);
        Assert.Equal(TaskbarAppearanceModes.Clear, restored.TaskbarFx.Mode);
        Assert.Equal(0x80FF00AAu, restored.TaskbarFx.Argb);
        Assert.False(restored.TaskbarFx.KeepAppearanceIfSuiteExits);
    }

    [Fact]
    public void Deserialize_fills_missing_nested_objects()
    {
        AppSettings restored = SettingsJson.Deserialize("""{"schemaVersion":1}""");
        Assert.NotNull(restored.Hotkey);
        Assert.NotNull(restored.NetSpeed);
        Assert.NotNull(restored.Capture);
        Assert.NotNull(restored.TaskbarFx);
        Assert.False(restored.TaskbarFx.Enabled);
        Assert.Equal(TaskbarAppearanceModes.Acrylic, restored.TaskbarFx.Mode);
        Assert.False(restored.Capture.SaveFileAfterCapture);
        Assert.False(restored.Capture.PinAfterCapture);
        Assert.True(restored.NetSpeed.EmbedInTaskbar);
        Assert.Equal("", restored.Capture.SaveDirectory);
        Assert.Equal(HotkeyBinding.DefaultVirtualKey, restored.Hotkey.VirtualKey);
        Assert.Equal("F1", restored.Hotkey.ToDisplayString());
        Assert.Equal(4, restored.SchemaVersion);
    }

    [Fact]
    public void Invalid_virtual_key_falls_back_to_default()
    {
        AppSettings restored = SettingsJson.Deserialize("""{"hotkey":{"virtualKey":0}}""");
        Assert.Equal(HotkeyBinding.DefaultVirtualKey, restored.Hotkey.VirtualKey);
    }

    [Fact]
    public void Schema_has_no_screenshot_or_secret_fields()
    {
        string[] forbidden = ["Image", "Bitmap", "Pixel", "Png", "Secret", "Password", "Token", "KeyMaterial"];
        IEnumerable<string> names = typeof(AppSettings)
            .GetProperties()
            .Select(p => p.Name)
            .Concat(typeof(NetSpeedSettings).GetProperties().Select(p => p.Name))
            .Concat(typeof(HotkeyBinding).GetProperties().Select(p => p.Name))
            .Concat(typeof(CaptureSettings).GetProperties().Select(p => p.Name))
            .Concat(typeof(TaskbarFxSettings).GetProperties().Select(p => p.Name))
            .Concat(typeof(UpdateSettings).GetProperties().Select(p => p.Name))
            .Concat(typeof(AdvancedSettings).GetProperties().Select(p => p.Name));

        foreach (string name in names)
        {
            Assert.DoesNotContain(forbidden, f => name.Contains(f, StringComparison.OrdinalIgnoreCase));
        }

        using JsonDocument doc = JsonDocument.Parse(SettingsJson.Serialize(AppSettings.CreateDefault()));
        string json = doc.RootElement.GetRawText();
        Assert.DoesNotContain("pixel", json, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("secret", json, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("password", json, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Empty_save_directory_resolves_under_pictures_or_localappdata()
    {
        string resolved = SettingsPaths.ResolveCaptureDirectory("  ");
        Assert.False(string.IsNullOrWhiteSpace(resolved));
        Assert.Contains("Suite", resolved, StringComparison.OrdinalIgnoreCase);
        Assert.Equal(@"D:\shots", SettingsPaths.ResolveCaptureDirectory(@"D:\shots"));
    }

    [Fact]
    public void Settings_path_is_localappdata_suite_json()
    {
        Assert.Equal("settings.json", Path.GetFileName(SettingsPaths.FilePath));
        Assert.Equal("Suite", Path.GetFileName(SettingsPaths.DirectoryPath));
        Assert.Contains(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            SettingsPaths.DirectoryPath,
            StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Schema_3_missing_appearance_fields_use_defaults()
    {
        AppSettings restored = SettingsJson.Deserialize(
            """{"schemaVersion":3,"netSpeed":{"visible":true,"embedInTaskbar":true}}""");
        Assert.Equal(13, restored.NetSpeed.FontSizeDip);
        Assert.True(restored.NetSpeed.ShowDownload);
        Assert.True(restored.NetSpeed.ShowUpload);
        Assert.True(restored.NetSpeed.BackgroundTransparent);
        Assert.False(restored.NetSpeed.ShowBorder);
        Assert.True(restored.NetSpeed.TwoLine);
        Assert.False(restored.NetSpeed.Bold);
        Assert.Null(restored.NetSpeed.DownColorArgb);
        Assert.Null(restored.NetSpeed.UpColorArgb);
        Assert.Equal(4, restored.SchemaVersion);
    }

    [Fact]
    public void Schema_4_roundtrip_update_advanced_and_capture_p2()
    {
        var original = AppSettings.CreateDefault();
        original.Update.Channel = UpdateSettings.ChannelPreview;
        original.Advanced.LoggingEnabled = true;
        original.Capture.ShowMagnifier = false;
        original.Capture.HistoryEnabled = false;
        original.Capture.HistoryMax = 12;
        original.NetSpeed.EmbedSecondary = true;
        original.Capture.ColorPickHotkey = new HotkeyBinding { VirtualKey = 0x71 };
        original.Capture.PinClickThroughHotkey = HotkeyBinding.DefaultPinClickThrough.Clone();
        original.Capture.ScrollCaptureHotkey = new HotkeyBinding { VirtualKey = 0x73 };
        AppSettings restored = SettingsJson.Deserialize(SettingsJson.Serialize(original));
        Assert.Equal(UpdateSettings.ChannelPreview, restored.Update.Channel);
        Assert.True(restored.Advanced.LoggingEnabled);
        Assert.False(restored.Capture.ShowMagnifier);
        Assert.False(restored.Capture.HistoryEnabled);
        Assert.Equal(12, restored.Capture.HistoryMax);
        Assert.True(restored.NetSpeed.EmbedSecondary);
        Assert.Equal(0x71, restored.Capture.ColorPickHotkey.VirtualKey);
        Assert.True(restored.Capture.PinClickThroughHotkey.Control);
        Assert.True(restored.Capture.PinClickThroughHotkey.Shift);
        Assert.Equal(0x51, restored.Capture.PinClickThroughHotkey.VirtualKey);
        Assert.Equal(0x73, restored.Capture.ScrollCaptureHotkey.VirtualKey);
        Assert.Equal(4, restored.SchemaVersion);
    }

    [Fact]
    public void Deserialize_clamps_font_and_restores_blank_lines()
    {
        AppSettings restored = SettingsJson.Deserialize(
            """{"schemaVersion":3,"netSpeed":{"fontSizeDip":99,"showDownload":false,"showUpload":false}}""");
        Assert.Equal(18, restored.NetSpeed.FontSizeDip);
        Assert.True(restored.NetSpeed.ShowDownload);
        Assert.True(restored.NetSpeed.ShowUpload);
    }

    [Fact]
    public void NetSpeedSettings_VisibleLineCount_respects_toggles()
    {
        var s = new NetSpeedSettings { ShowDownload = true, ShowUpload = true, TwoLine = true };
        Assert.Equal(2, s.VisibleLineCount());
        s.ShowUpload = false;
        Assert.Equal(1, s.VisibleLineCount());
        s.TwoLine = false;
        s.ShowUpload = true;
        Assert.Equal(1, s.VisibleLineCount());
    }
}
