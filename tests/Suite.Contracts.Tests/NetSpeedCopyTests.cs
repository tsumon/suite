using Suite.Contracts;

namespace Suite.Contracts.Tests;

public sealed class NetSpeedCopyTests
{
    [Fact]
    public void Embed_failed_sentence_uses_spec_prefix_and_reason()
    {
        Assert.Equal(
            "无法钉到任务栏，已改用桌面悬浮窗。任务栏布局不允许嵌入。",
            NetSpeedCopy.EmbedFailed("Win11 居中栏/小组件把槽挤没"));
    }

    [Theory]
    [InlineData("找不到任务栏（Shell_TrayWnd）。", "找不到任务栏。")]
    [InlineData("SetParent 未生效（Win32 5）。", "系统不允许把网速嵌进去。")]
    [InlineData("无法摆放任务栏网速窗。", "任务栏里没有足够位置。")]
    [InlineData("无法读取任务栏客户区。", "无法读取任务栏大小。")]
    [InlineData("任务栏布局不允许嵌入。", "任务栏布局不允许嵌入。")]
    public void Reasons_are_human_not_win32_codes(string raw, string expected)
    {
        Assert.Equal(expected, NetSpeedCopy.MapReason(raw));
        Assert.DoesNotContain("Win32 5", NetSpeedCopy.EmbedFailed(raw), StringComparison.Ordinal);
    }

    [Fact]
    public void At_least_one_line_copy_is_stable()
    {
        Assert.Equal("至少显示上行或下行。", NetSpeedCopy.AtLeastOneLine);
        Assert.Equal("只改任务栏里的字。桌面悬浮窗仍是 13。", NetSpeedCopy.FontSizeHelp);
    }
}
