use crate::settings::{TaskbarAppearanceMode, TaskbarAppearanceModes};

pub struct HotkeyCopy;

impl HotkeyCopy {
    pub const ERROR_ALREADY_REGISTERED: u32 = 1409;
    pub const ERROR_ACCESS_DENIED: u32 = 5;
    pub const ERROR_INVALID_PARAMETER: u32 = 87;

    pub const TRY_NEXT_BUTTON: &'static str = "试用下一可用键";
    pub const HELP_LINE: &'static str = "默认 F1。失败会提示占用原因，并建议空闲 F 键。不会反复抢键。";
    pub const NO_FREE_KEY: &'static str = "F1–F12 与字母键都已被占用。请关掉冲突程序，或加上 Ctrl/Shift 再试。";
    pub const APPLIED_NEXT: &'static str = "已改用 ";

    pub fn map_win32(code: u32) -> String {
        match code {
            Self::ERROR_ALREADY_REGISTERED => "已被其他程序占用。".to_string(),
            Self::ERROR_ACCESS_DENIED => "系统不允许注册（权限或策略）。".to_string(),
            Self::ERROR_INVALID_PARAMETER => "按键组合无效。".to_string(),
            0 => "原因不明。".to_string(),
            _ => format!("系统错误 {code}。"),
        }
    }

    pub fn format_suggestions(suggestions: &[String]) -> String {
        suggestions.join("、")
    }

    pub fn capture_failed(display: &str, win32_error: u32, suggestions: &[String]) -> String {
        let reason = Self::map_win32(win32_error);
        let alts = Self::format_suggestions(suggestions);
        if alts.is_empty() {
            format!("热键 {display} 注册失败。{reason} 请在设置中改键。")
        } else {
            format!("热键 {display} 注册失败。{reason} 可改用 {alts}。")
        }
    }

    pub fn pin_failed(win32_error: u32, suggestions: &[String]) -> String {
        let reason = Self::map_win32(win32_error);
        let alts = Self::format_suggestions(suggestions);
        if alts.is_empty() {
            format!("热键 F3（钉剪贴板）注册失败。{reason}")
        } else {
            format!("热键 F3（钉剪贴板）注册失败。{reason} 截图键可改用 {alts}。")
        }
    }

    pub fn applied_alternate(display: &str) -> String {
        format!("{}{display}。", Self::APPLIED_NEXT)
    }
}

pub struct NetSpeedCopy;

impl NetSpeedCopy {
    pub const EMBED_FAILED_PREFIX: &'static str = "无法钉到任务栏，已改用桌面悬浮窗。";
    pub const REASON_NO_TRAY: &'static str = "找不到任务栏。";
    pub const REASON_SET_PARENT: &'static str = "系统不允许把网速嵌进去。";
    pub const REASON_NO_SLOT: &'static str = "任务栏里没有足够位置。";
    pub const REASON_NO_CLIENT: &'static str = "无法读取任务栏大小。";
    pub const REASON_LAYOUT: &'static str = "任务栏布局不允许嵌入。";
    pub const EMBED_HELP: &'static str = "默认嵌进主任务栏。嵌不进去会告诉你，并改用可拖的桌面悬浮窗。取消勾选则只用悬浮窗。";
    pub const AT_LEAST_ONE_LINE: &'static str = "至少显示上行或下行。";
    pub const FONT_SIZE_HELP: &'static str = "只改任务栏里的字。桌面悬浮窗仍是 13。";

    pub fn map_reason(raw: Option<&str>) -> String {
        let text = raw.unwrap_or("").trim();
        if text.is_empty() {
            return String::new();
        }
        let lower = text.to_ascii_lowercase();
        if text.contains("找不到任务栏") || lower.contains("shell_traywnd") || text.contains("任务栏已消失") {
            return Self::REASON_NO_TRAY.to_string();
        }
        if text.contains("布局") || text.contains("小组件") || text.contains("挤") {
            return Self::REASON_LAYOUT.to_string();
        }
        if lower.contains("setparent") || text.contains("不允许") {
            return Self::REASON_SET_PARENT.to_string();
        }
        if text.contains("客户区") || text.contains("大小") {
            return Self::REASON_NO_CLIENT.to_string();
        }
        if text.contains("摆放") || text.contains("位置") || text.contains("过小") {
            return Self::REASON_NO_SLOT.to_string();
        }
        if lower.starts_with("win32") || (text.len() <= 8 && text.trim_end_matches(&['。', ')', '（', '('][..]).parse::<i32>().is_ok()) {
            return Self::REASON_SET_PARENT.to_string();
        }
        if text.ends_with('。') {
            text.to_string()
        } else {
            format!("{text}。")
        }
    }

    pub fn embed_failed(reason: Option<&str>) -> String {
        let mapped = Self::map_reason(reason);
        if mapped.is_empty() {
            Self::EMBED_FAILED_PREFIX.to_string()
        } else {
            format!("{}{mapped}", Self::EMBED_FAILED_PREFIX)
        }
    }
}

pub struct TaskbarFxCopy;

impl TaskbarFxCopy {
    pub const WIN11_UNDELIVERED: &'static str = "无法应用任务栏效果。截图和网速仍可用。";
    pub const APPLIED: &'static str = "任务栏效果已应用。";
    pub const CLOSED: &'static str = "任务栏效果已关闭。";
    pub const INIT_FAILED: &'static str = "无法启用任务栏效果。截图和网速仍可用。";
    pub const BLOCKED_BY_SECURITY: &'static str = "无法应用任务栏效果：安全软件可能拦截了操作。截图和网速仍可用。";
    pub const STILL_AVAILABLE: &'static str = "截图和网速仍可用。";
    pub const PENDING_FILLS: &'static str = "暂时找不到任务栏背景，请再点一次应用。";
    pub const ENABLE_HELP: &'static str = "与 Suite 同一进程内的任务栏效果模块。关掉并点应用，恢复系统默认。不要结束 explorer。";
    pub const COLOR_HELP: &'static str = "实色会忽略透明度。全透明和亚克力才用上面的透明度。";

    pub fn mode_label(wire: Option<&str>) -> &'static str {
        match TaskbarAppearanceModes::try_parse(wire).unwrap_or(TaskbarAppearanceMode::Normal) {
            TaskbarAppearanceMode::Opaque => "实色",
            TaskbarAppearanceMode::Clear => "全透明",
            TaskbarAppearanceMode::Acrylic => "亚克力",
            TaskbarAppearanceMode::Normal => "系统默认",
        }
    }

    pub fn mode_help(wire: Option<&str>) -> &'static str {
        match TaskbarAppearanceModes::try_parse(wire).unwrap_or(TaskbarAppearanceMode::Normal) {
            TaskbarAppearanceMode::Opaque => "任务栏铺满所选颜色，不透明。",
            TaskbarAppearanceMode::Clear => "任务栏完全看穿。图标还在。",
            TaskbarAppearanceMode::Acrylic => "磨砂半透明。",
            TaskbarAppearanceMode::Normal => "不改任务栏外观。",
        }
    }

    pub fn apply_failed(reason: Option<&str>) -> String {
        let raw = reason.unwrap_or("").trim();
        if raw.contains("安全软件") {
            return Self::BLOCKED_BY_SECURITY.to_string();
        }
        if raw.is_empty() {
            return format!("无法应用任务栏效果。{}", Self::STILL_AVAILABLE);
        }
        let text = if raw.ends_with('。') {
            raw.to_string()
        } else {
            format!("{raw}。")
        };
        if text.contains(Self::STILL_AVAILABLE) {
            if text.starts_with("无法应用任务栏效果。") {
                text
            } else {
                format!("无法应用任务栏效果。{text}")
            }
        } else {
            format!("无法应用任务栏效果。{text}{}", Self::STILL_AVAILABLE)
        }
    }
}
