using Suite.NetSpeed;

namespace Suite.NetSpeed.Tests;

public sealed class TaskbarSlotChromeTests
{
    [Fact]
    public void OffsetY_centers_two_lines_in_48px_tray_at_96dpi()
    {
        // window 48 DIP, font 13, lineHeight 1.05, 2 lines → content 27.3; leftover 20.7; offset = 10.35 + 1
        double offset = TaskbarSlotChrome.OffsetYDip(windowHeightDip: 48, fontSizeDip: 13, visibleLineCount: 2);
        Assert.Equal((48 - (13 * 1.05 * 2)) / 2 + 1, offset, 6);
        Assert.True(offset > 8, "must not sit near the top like joe-netspeed-ugly.png");
    }

    [Fact]
    public void OffsetY_is_zero_when_content_taller_than_slot()
    {
        double offset = TaskbarSlotChrome.OffsetYDip(windowHeightDip: 20, fontSizeDip: 18, visibleLineCount: 2);
        Assert.Equal(0, offset);
    }

    [Fact]
    public void OffsetY_single_line_still_centered()
    {
        double offset = TaskbarSlotChrome.OffsetYDip(windowHeightDip: 48, fontSizeDip: 13, visibleLineCount: 1);
        Assert.Equal((48 - (13 * 1.05)) / 2 + 1, offset, 6);
    }

    [Fact]
    public void ClampFontSize_bounds_10_to_18()
    {
        Assert.Equal(10, TaskbarSlotChrome.ClampFontSize(3));
        Assert.Equal(18, TaskbarSlotChrome.ClampFontSize(40));
        Assert.Equal(13, TaskbarSlotChrome.ClampFontSize(13));
    }

    [Fact]
    public void Hardcoded_40dip_window_with_11px_top_pad_is_the_ugly_anti_pattern()
    {
        // Document why Height=40 + FontSize=11 + top padding failed Joe's screenshot.
        double content = TaskbarSlotChrome.ContentHeightDip(11, 2);
        double leftoverIfLocked40 = 40 - content;
        Assert.True(leftoverIfLocked40 > 12, "old 40 DIP + 11px left a large empty band");
        double centered = TaskbarSlotChrome.OffsetYDip(48, 13, 2);
        Assert.True(centered > leftoverIfLocked40 / 4);
    }
}
