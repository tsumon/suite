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
}
