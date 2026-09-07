using Suite.Contracts;

namespace Suite.Contracts.Tests;

public sealed class TaskbarFxCopyTests
{
    [Theory]
    [InlineData("normal", "系统默认")]
    [InlineData("opaque", "实色")]
    [InlineData("clear", "全透明")]
    [InlineData("acrylic", "亚克力")]
    [InlineData("ACRYLIC", "亚克力")]
    public void Mode_labels_are_chinese_only(string wire, string expected)
    {
        string label = TaskbarFxCopy.ModeLabel(wire);
        Assert.Equal(expected, label);
        Assert.DoesNotContain("Normal", label, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("Opaque", label, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("Clear", label, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("Acrylic", label, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("(", label, StringComparison.Ordinal);
    }

    [Fact]
    public void Win11_xaml_path_can_show_applied()
    {
        string text = TaskbarFxCopy.FormatStatus(new TaskbarFxStatusDto
        {
            Enabled = true,
            Path = TaskbarFxIpc.PathWin11Xaml,
            Message = TaskbarFxCopy.Applied,
        });
        Assert.Equal(TaskbarFxCopy.Applied, text);
        Assert.DoesNotContain("未交付", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Win11_xaml_path_failure_keeps_capture_available()
    {
        string text = TaskbarFxCopy.FormatStatus(new TaskbarFxStatusDto
        {
            Enabled = true,
            Path = TaskbarFxIpc.PathWin11Xaml,
            Message = TaskbarFxCopy.ApplyFailed("任务栏效果被安全软件拦住，或 Win11 辅助模块没编出来。"),
        });
        Assert.Contains("无法应用任务栏效果", text, StringComparison.Ordinal);
        Assert.Contains("截图和网速仍可用", text, StringComparison.Ordinal);
        Assert.DoesNotContain("未交付", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Closed_and_applied_match_spec()
    {
        Assert.Equal(
            TaskbarFxCopy.Closed,
            TaskbarFxCopy.FormatStatus(new TaskbarFxStatusDto { Enabled = false, Path = TaskbarFxIpc.PathWin10Swca, Message = TaskbarFxCopy.Closed }));
        Assert.Equal(
            TaskbarFxCopy.Applied,
            TaskbarFxCopy.FormatStatus(new TaskbarFxStatusDto { Enabled = true, Path = TaskbarFxIpc.PathWin10Swca, Message = TaskbarFxCopy.Applied }));
    }

    [Fact]
    public void Apply_failed_keeps_capture_available_and_skips_explorer_advice()
    {
        string text = TaskbarFxCopy.ApplyFailed("调用失败");
        Assert.StartsWith("无法应用任务栏效果。", text, StringComparison.Ordinal);
        Assert.Contains("截图和网速仍可用", text, StringComparison.Ordinal);
        Assert.DoesNotContain("explorer", text, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("DLL", text, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("SWCA", text, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Pending_fills_timeout_is_chinese_and_keeps_capture()
    {
        string text = TaskbarFxCopy.ApplyFailed(TaskbarFxCopy.PendingFills);
        Assert.Contains("暂时找不到任务栏背景", text, StringComparison.Ordinal);
        Assert.Contains("请再点一次应用", text, StringComparison.Ordinal);
        Assert.Contains("截图和网速仍可用", text, StringComparison.Ordinal);
        Assert.DoesNotContain("background node", text, StringComparison.OrdinalIgnoreCase);
        Assert.DoesNotContain("visual tree", text, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Enable_help_says_same_process_and_known_tradeoff()
    {
        Assert.Contains("同一进程", TaskbarFxCopy.EnableHelp, StringComparison.Ordinal);
        Assert.Contains("不要结束 explorer", TaskbarFxCopy.EnableHelp, StringComparison.Ordinal);
        Assert.Contains("截图也可能一起挂", TaskbarFxCopy.EnableHelp, StringComparison.Ordinal);
        Assert.DoesNotContain("TaskbarFx.exe", TaskbarFxCopy.EnableHelp, StringComparison.Ordinal);
        Assert.DoesNotContain("管道", TaskbarFxCopy.EnableHelp, StringComparison.Ordinal);
    }
}