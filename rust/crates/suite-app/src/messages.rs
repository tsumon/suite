use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

use suite_capture::{OcrResult, PixelBuffer, PixelRect};
use suite_taskbarfx::TaskbarFxStatus;

pub const WM_APP_TRAY_CALLBACK: u32 = WM_APP + 1;
pub const WM_APP_NETSPEED_SAMPLE: u32 = WM_APP + 2;
pub const WM_APP_TASKBAR_FX_RESULT: u32 = WM_APP + 3;
pub const WM_APP_OCR_DONE: u32 = WM_APP + 4;
pub const WM_APP_SCROLL_PROGRESS: u32 = WM_APP + 5;
pub const WM_APP_SCROLL_DONE: u32 = WM_APP + 6;
pub const WM_APP_UPDATE_CHECK_DONE: u32 = WM_APP + 7;
pub const WM_APP_COMMAND: u32 = WM_APP + 8;
pub const WM_APP_NETSPEED_POS: u32 = WM_APP + 9;
pub const WM_APP_CAPTURE_DONE: u32 = WM_APP + 10;
pub const WM_APP_COLOR_PICK_DONE: u32 = WM_APP + 11;
pub const WM_APP_SCROLL_PICKED: u32 = WM_APP + 12;
pub const WM_APP_SETTINGS_APPLY: u32 = WM_APP + 13;
pub const WM_APP_SETTINGS_CLOSED: u32 = WM_APP + 14;
pub const WM_APP_HISTORY_CLOSED: u32 = WM_APP + 15;
pub const WM_APP_PIN_CHANGED: u32 = WM_APP + 16;
pub const WM_APP_PIN_MANAGE_CLOSED: u32 = WM_APP + 17;
pub const WM_APP_TRAY_BALLOON: u32 = WM_APP + 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum AppCommand {
    ToggleTheme = 1001,
    ToggleNetSpeed = 1002,
    Capture = 1003,
    ScrollCapture = 1004,
    ColorPick = 1005,
    History = 1006,
    PinFromClipboard = 1007,
    PinManage = 1008,
    ClearClickThrough = 1009,
    OpenSettings = 1010,
    ToggleRunAtStartup = 1011,
    Exit = 1012,
    RequestFloatNetSpeed = 1013,
    RequestEmbedNetSpeed = 1014,
    RefreshAdaptersAndSettings = 1015,
    CheckUpdates = 1016,
    RestoreDefaults = 1017,
    TryNextHotkey = 1018,
    UseAcrylicReadable = 1019,
    PinHistoryPath = 1020,
    FocusPin = 1021,
    TogglePinThrough = 1022,
    ClosePin = 1023,
    CloseAllPins = 1024,
    DelayedCapture = 1025,
    TogglePinsVisible = 1026,
}

impl AppCommand {
    pub fn from_wparam(wparam: WPARAM) -> Option<Self> {
        match wparam.0 {
            1001 => Some(Self::ToggleTheme),
            1002 => Some(Self::ToggleNetSpeed),
            1003 => Some(Self::Capture),
            1004 => Some(Self::ScrollCapture),
            1005 => Some(Self::ColorPick),
            1006 => Some(Self::History),
            1007 => Some(Self::PinFromClipboard),
            1008 => Some(Self::PinManage),
            1009 => Some(Self::ClearClickThrough),
            1010 => Some(Self::OpenSettings),
            1011 => Some(Self::ToggleRunAtStartup),
            1012 => Some(Self::Exit),
            1013 => Some(Self::RequestFloatNetSpeed),
            1014 => Some(Self::RequestEmbedNetSpeed),
            1015 => Some(Self::RefreshAdaptersAndSettings),
            1016 => Some(Self::CheckUpdates),
            1017 => Some(Self::RestoreDefaults),
            1018 => Some(Self::TryNextHotkey),
            1019 => Some(Self::UseAcrylicReadable),
            1020 => Some(Self::PinHistoryPath),
            1021 => Some(Self::FocusPin),
            1022 => Some(Self::TogglePinThrough),
            1023 => Some(Self::ClosePin),
            1024 => Some(Self::CloseAllPins),
            1025 => Some(Self::DelayedCapture),
            1026 => Some(Self::TogglePinsVisible),
            _ => None,
        }
    }
}

pub struct TrayBalloonMsg {
    pub title: String,
    pub text: String,
}

pub struct TaskbarFxApplyMsg {
    pub gen_id: u64,
    pub status: TaskbarFxStatus,
    pub was_enabled: bool,
    pub shutdown_when_disabled: bool,
}

pub struct OcrDoneMsg {
    pub overlay_hwnd: isize,
    pub result: OcrResult,
}

pub struct CaptureDoneMsg {
    pub cancelled: bool,
    pub pin_requested: bool,
    pub save_requested: bool,
    pub image: Option<PixelBuffer>,
    pub selection: PixelRect,
    pub dpi_x: u32,
    pub dpi_y: u32,
    pub error: Option<String>,
}

pub struct ColorPickDoneMsg {
    pub copied_value: Option<String>,
    pub error: Option<String>,
}

pub struct ScrollPickMsg {
    pub rect: Option<PixelRect>,
}

pub struct NetSpeedPosMsg {
    pub left: f64,
    pub top: f64,
}

pub struct UpdateCheckResult {
    pub ok: bool,
    pub message: String,
    pub version: Option<String>,
    pub html_url: Option<String>,
    pub channel_zh: Option<String>,
    pub has_update: bool,
}

pub fn post_boxed<T>(hwnd: HWND, msg: u32, wparam: usize, payload: T) {
    if hwnd.is_invalid() {
        return;
    }
    let ptr = Box::into_raw(Box::new(payload));
    let res = unsafe { PostMessageW(Some(hwnd), msg, WPARAM(wparam), LPARAM(ptr as isize)) };
    if res.is_err() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

pub unsafe fn take_boxed<T>(lparam: LPARAM) -> Option<Box<T>> {
    if lparam.0 == 0 {
        None
    } else {
        Some(unsafe { Box::from_raw(lparam.0 as *mut T) })
    }
}

pub fn post_command(hwnd: HWND, cmd: AppCommand, lparam: isize) {
    if !hwnd.is_invalid() {
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_APP_COMMAND, WPARAM(cmd as usize), LPARAM(lparam));
        }
    }
}
