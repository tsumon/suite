use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, SendNotifyMessageW, WM_SETTINGCHANGE};

use crate::constants::{IMMERSIVE_COLOR_SET_STR, to_wide_null};

/// Notify top-level windows of ImmersiveColorSet without blocking the caller.
/// `SendNotifyMessageW(HWND_BROADCAST)` returns immediately instead of hanging per hung window.
pub fn send_immersive_color_set() -> Result<(), String> {
    let param = to_wide_null(IMMERSIVE_COLOR_SET_STR);
    let ok = unsafe {
        SendNotifyMessageW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(param.as_ptr() as isize),
        )
    };
    if ok.is_ok() {
        Ok(())
    } else {
        Err(format!(
            "SendNotifyMessageW failed for ImmersiveColorSet: {}",
            windows::core::Error::from_win32()
        ))
    }
}
