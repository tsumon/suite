namespace Suite.Contracts;

/// <summary>
/// Chinese copy for RegisterHotKey failures. Spec: settings must say what happened, not Win32 jargon.
/// </summary>
public static class HotkeyCopy
{
    /// <summary>ERROR_HOTKEY_ALREADY_REGISTERED</summary>
    public const int ErrorAlreadyRegistered = 1409;

    /// <summary>ERROR_ACCESS_DENIED</summary>
    public const int ErrorAccessDenied = 5;

    /// <summary>ERROR_INVALID_PARAMETER / bad vk</summary>
    public const int ErrorInvalidParameter = 87;

    public const string TryNextButton = "试用下一可用键";
    public const string HelpLine = "默认 F1。失败会提示占用原因，并建议空闲 F 键。不会反复抢键。";
    public const string NoFreeKey = "F1–F12 与字母键都已被占用。请关掉冲突程序，或加上 Ctrl/Shift 再试。";
    public const string AppliedNext = "已改用 ";

    public static string CaptureFailed(string display, int win32Error, IReadOnlyList<string>? suggestions = null)
    {
        string reason = MapWin32(win32Error);
        string body = "热键 " + display + " 注册失败。" + reason;
        string alts = FormatSuggestions(suggestions);
        if (alts.Length > 0)
        {
            body += " 可改用 " + alts + "。";
        }
        else
        {
            body += " 请在设置中改键。";
        }

        return body;
    }

    public static string PinFailed(int win32Error, IReadOnlyList<string>? suggestions = null)
    {
        string reason = MapWin32(win32Error);
        string body = "热键 F3（钉剪贴板）注册失败。" + reason;
        string alts = FormatSuggestions(suggestions);
        if (alts.Length > 0)
        {
            body += " 截图键可改用 " + alts + "。";
        }

        return body;
    }

    public static string MapWin32(int code) => code switch
    {
        ErrorAlreadyRegistered => "已被其他程序占用。",
        ErrorAccessDenied => "系统不允许注册（权限或策略）。",
        ErrorInvalidParameter => "按键组合无效。",
        0 => "原因不明。",
        _ => "系统错误 " + code + "。",
    };

    public static string FormatSuggestions(IReadOnlyList<string>? suggestions)
    {
        if (suggestions is null || suggestions.Count == 0)
        {
            return "";
        }

        return string.Join("、", suggestions);
    }

    public static string AppliedAlternate(string display) => AppliedNext + display + "。";
}
