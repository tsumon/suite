namespace Suite.Contracts;

/// <summary>
/// Settings / balloon copy for taskbar appearance. Chinese labels only; no DLL/pipe/TAP/SWCA.
/// </summary>
public static class TaskbarFxCopy
{
    public const string Win11Undelivered = "无法应用任务栏效果。截图和网速仍可用。";
    public const string Applied = "任务栏效果已应用。";
    public const string Closed = "任务栏效果已关闭。";
    public const string InitFailed = "无法启用任务栏效果。截图和网速仍可用。";
    public const string BlockedBySecurity = "任务栏效果被安全软件拦住。截图和网速仍可用。";
    public const string StillAvailable = "截图和网速仍可用。";
    public const string PendingFills = "暂时找不到任务栏背景，请再点一次应用。";
    public const string EnableHelp =
        "与 Suite 同一进程内的任务栏效果模块。关掉并点应用，恢复系统默认。不要结束 explorer。透明挂住时截图也可能一起挂（已知取舍）。";
    public const string ColorHelp = "实色会忽略透明度。全透明和亚克力才用上面的透明度。";

    public static string ModeLabel(string? wire)
    {
        TaskbarAppearanceModes.TryParse(wire, out TaskbarAppearanceMode mode);
        return mode switch
        {
            TaskbarAppearanceMode.Opaque => "实色",
            TaskbarAppearanceMode.Clear => "全透明",
            TaskbarAppearanceMode.Acrylic => "亚克力",
            _ => "系统默认",
        };
    }

    public static string ModeHelp(string? wire)
    {
        TaskbarAppearanceModes.TryParse(wire, out TaskbarAppearanceMode mode);
        return mode switch
        {
            TaskbarAppearanceMode.Opaque => "任务栏铺满所选颜色，不透明。",
            TaskbarAppearanceMode.Clear => "任务栏完全看穿。图标还在。",
            TaskbarAppearanceMode.Acrylic => "磨砂半透明。",
            _ => "不改任务栏外观。",
        };
    }

    public static string ApplyFailed(string? reason)
    {
        string text = (reason ?? "").Trim();
        if (text.Contains("安全软件", StringComparison.Ordinal))
        {
            return BlockedBySecurity;
        }

        if (string.IsNullOrEmpty(text))
        {
            return "无法应用任务栏效果。" + StillAvailable;
        }

        if (!text.EndsWith('。'))
        {
            text += "。";
        }

        if (text.Contains(StillAvailable, StringComparison.Ordinal))
        {
            return text.StartsWith("无法应用任务栏效果。", StringComparison.Ordinal)
                ? text
                : "无法应用任务栏效果。" + text;
        }

        return "无法应用任务栏效果。" + text + StillAvailable;
    }

    public static string FormatStatus(TaskbarFxStatusDto status)
    {
        if (!string.IsNullOrWhiteSpace(status.Message))
        {
            return status.Message;
        }

        if (!string.IsNullOrWhiteSpace(status.LastError))
        {
            return status.LastError;
        }

        return status.Enabled ? Applied : Closed;
    }
}