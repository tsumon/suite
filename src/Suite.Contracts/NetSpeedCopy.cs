namespace Suite.Contracts;

/// <summary>
/// Balloon / settings copy for net-speed taskbar embed. Spec: design/NETSPEED-TASKBAR-SPEC.md.
/// </summary>
public static class NetSpeedCopy
{
    public const string EmbedFailedPrefix = "无法钉到任务栏，已改用桌面悬浮窗。";
    public const string ReasonNoTray = "找不到任务栏。";
    public const string ReasonSetParent = "系统不允许把网速嵌进去。";
    public const string ReasonNoSlot = "任务栏里没有足够位置。";
    public const string ReasonNoClient = "无法读取任务栏大小。";
    public const string ReasonLayout = "任务栏布局不允许嵌入。";
    public const string EmbedHelp = "默认嵌进主任务栏。嵌不进去会告诉你，并改用可拖的桌面悬浮窗。取消勾选则只用悬浮窗。";
    public const string AtLeastOneLine = "至少显示上行或下行。";
    public const string FontSizeHelp = "只改任务栏里的字。桌面悬浮窗仍是 13。";

    public static string EmbedFailed(string? reason)
    {
        string mapped = MapReason(reason);
        return string.IsNullOrEmpty(mapped) ? EmbedFailedPrefix : EmbedFailedPrefix + " " + mapped;
    }

    public static string MapReason(string? raw)
    {
        string text = (raw ?? "").Trim();
        if (text.Length == 0)
        {
            return "";
        }

        if (text.Contains("找不到任务栏", StringComparison.Ordinal)
            || text.Contains("Shell_TrayWnd", StringComparison.OrdinalIgnoreCase)
            || text.Contains("任务栏已消失", StringComparison.Ordinal))
        {
            return ReasonNoTray;
        }

        if (text.Contains("SetParent", StringComparison.OrdinalIgnoreCase)
            || text.Contains("不允许", StringComparison.Ordinal))
        {
            return ReasonSetParent;
        }

        if (text.Contains("布局", StringComparison.Ordinal)
            || text.Contains("小组件", StringComparison.Ordinal)
            || text.Contains("挤", StringComparison.Ordinal))
        {
            return ReasonLayout;
        }

        if (text.Contains("客户区", StringComparison.Ordinal)
            || text.Contains("大小", StringComparison.Ordinal))
        {
            return ReasonNoClient;
        }

        if (text.Contains("摆放", StringComparison.Ordinal)
            || text.Contains("位置", StringComparison.Ordinal)
            || text.Contains("过小", StringComparison.Ordinal))
        {
            return ReasonNoSlot;
        }

        if (text.StartsWith("Win32", StringComparison.OrdinalIgnoreCase)
            || LooksLikeBareWin32(text))
        {
            return ReasonSetParent;
        }

        if (!text.EndsWith('。'))
        {
            text += "。";
        }

        return text;
    }

    private static bool LooksLikeBareWin32(string text)
    {
        if (text.StartsWith("Win32 ", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        return text.Length <= 8 && int.TryParse(text.TrimEnd('。', ')', '（', '('), out _);
    }
}