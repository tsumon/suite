using Suite.Contracts;

namespace Suite.Contracts.Tests;

public sealed class HotkeyCopyTests
{
    [Fact]
    public void Capture_failed_names_key_and_occupation()
    {
        string text = HotkeyCopy.CaptureFailed("F1", HotkeyCopy.ErrorAlreadyRegistered, ["F2", "F4"]);
        Assert.Contains("F1", text, StringComparison.Ordinal);
        Assert.Contains("占用", text, StringComparison.Ordinal);
        Assert.Contains("F2", text, StringComparison.Ordinal);
        Assert.Contains("F4", text, StringComparison.Ordinal);
        Assert.DoesNotContain("Oops", text, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Map_win32_uses_plain_chinese()
    {
        Assert.Equal("已被其他程序占用。", HotkeyCopy.MapWin32(HotkeyCopy.ErrorAlreadyRegistered));
        Assert.Equal("系统不允许注册（权限或策略）。", HotkeyCopy.MapWin32(HotkeyCopy.ErrorAccessDenied));
        Assert.Equal("试用下一可用键", HotkeyCopy.TryNextButton);
        Assert.Contains("F1–F12", HotkeyCopy.NoFreeKey, StringComparison.Ordinal);
    }

    [Fact]
    public void Applied_alternate_is_result_sentence()
    {
        Assert.Equal("已改用 F2。", HotkeyCopy.AppliedAlternate("F2"));
    }
}
