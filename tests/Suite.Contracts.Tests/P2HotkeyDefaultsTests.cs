using Suite.Contracts;

namespace Suite.Contracts.Tests;

public sealed class P2HotkeyDefaultsTests
{
    [Fact]
    public void Color_pick_default_is_f2_and_click_through_is_ctrl_shift_q()
    {
        Assert.Equal(0x71, HotkeyBinding.DefaultColorPickVirtualKey);
        Assert.Equal("F2", HotkeyBinding.DefaultColorPick.ToDisplayString());
        Assert.Equal("Ctrl+Shift+Q", HotkeyBinding.DefaultPinClickThrough.ToDisplayString());
        Assert.False(HotkeyBinding.DefaultColorPick.IsDisabled);
        Assert.True(new HotkeyBinding { VirtualKey = 0 }.IsDisabled);
    }

    [Fact]
    public void Scroll_capture_default_is_f4_and_virtual_key_zero_disables()
    {
        Assert.Equal(0x73, HotkeyBinding.DefaultScrollCaptureVirtualKey);
        Assert.Equal("F4", HotkeyBinding.DefaultScrollCapture.ToDisplayString());
        Assert.False(HotkeyBinding.DefaultScrollCapture.IsDisabled);
        Assert.Equal(0x73, new CaptureSettings().ScrollCaptureHotkey.VirtualKey);
        Assert.True(new HotkeyBinding { VirtualKey = 0 }.IsDisabled);
    }

    [Fact]
    public void Scroll_capture_hotkey_roundtrip_and_null_coalesce_to_f4()
    {
        var original = AppSettings.CreateDefault();
        original.Capture.ScrollCaptureHotkey = new HotkeyBinding { VirtualKey = 0x74 }; // F5
        AppSettings restored = SettingsJson.Deserialize(SettingsJson.Serialize(original));
        Assert.Equal(0x74, restored.Capture.ScrollCaptureHotkey.VirtualKey);

        AppSettings disabled = SettingsJson.Deserialize(
            """{"schemaVersion":4,"capture":{"scrollCaptureHotkey":{"virtualKey":0}}}""");
        Assert.Equal(0, disabled.Capture.ScrollCaptureHotkey.VirtualKey);
        Assert.True(disabled.Capture.ScrollCaptureHotkey.IsDisabled);

        AppSettings missing = SettingsJson.Deserialize("""{"schemaVersion":4,"capture":{}}""");
        Assert.Equal(HotkeyBinding.DefaultScrollCaptureVirtualKey, missing.Capture.ScrollCaptureHotkey.VirtualKey);
        Assert.Equal("F4", missing.Capture.ScrollCaptureHotkey.ToDisplayString());
    }
}
