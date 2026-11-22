use windows::core::PCWSTR;

pub const WM_SETTINGCHANGE_MSG: u32 = 0x001A;
pub const WM_HOTKEY_MSG: u32 = 0x0312;

pub const IMMERSIVE_COLOR_SET_STR: &str = "ImmersiveColorSet";
pub const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const RUN_VALUE_NAME: &str = "Suite";
pub const PERSONALIZE_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
pub const APPS_USE_LIGHT_THEME: &str = "AppsUseLightTheme";
pub const SYSTEM_USES_LIGHT_THEME: &str = "SystemUsesLightTheme";

pub const SINGLE_INSTANCE_MUTEX_NAME: &str = r"Local\Suite.App.SingleInstance";
pub const TASKBAR_FX_TAP_FILE_NAME: &str = "TaskbarFx.Tap.dll";

pub const HOTKEY_ID_CAPTURE: i32 = 1;
pub const HOTKEY_ID_PIN_CLIPBOARD: i32 = 2;
pub const HOTKEY_ID_COLOR_PICK: i32 = 3;
pub const HOTKEY_ID_PIN_CLICK_THROUGH: i32 = 4;
pub const HOTKEY_ID_SCROLL_CAPTURE: i32 = 5;

pub fn to_wide_null(s: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0);
    wide
}

/// Converts a NUL-terminated UTF-16 pointer into a Rust `String`.
///
/// # Safety
/// `ptr` must either be null or point to a valid NUL-terminated UTF-16 string.
pub unsafe fn wide_ptr_to_string(ptr: PCWSTR) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { ptr.to_string().unwrap_or_default() }
}
