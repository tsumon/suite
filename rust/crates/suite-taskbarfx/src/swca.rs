use core::ffi::c_void;
use suite_contracts::TaskbarAppearanceMode;
use suite_platform::TaskbarProbe;
use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::core::{BOOL, PCSTR, PCWSTR};

use crate::protocol::TaskbarFxResultCode;

const WCA_ACCENT_POLICY: u32 = 19;

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum AccentState {
    Disabled = 0,
    Gradient = 1,
    TransparentGradient = 2,
    AcrylicBlurBehind = 4,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    gradient_abgr: u32,
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttributeData {
    attribute: u32,
    data: *mut c_void,
    data_size: u32,
}

type SetWindowCompositionAttributeFn =
    unsafe extern "system" fn(HWND, *mut WindowCompositionAttributeData) -> BOOL;

fn argb_to_abgr(argb: u32) -> u32 {
    let a = (argb >> 24) & 0xFF;
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    (a << 24) | (b << 16) | (g << 8) | r
}

fn load_swca() -> Option<SetWindowCompositionAttributeFn> {
    let user32_name = to_wide_null("user32.dll");
    let module = unsafe {
        GetModuleHandleW(PCWSTR(user32_name.as_ptr()))
            .or_else(|_| LoadLibraryW(PCWSTR(user32_name.as_ptr())))
            .ok()?
    };
    let proc = unsafe {
        GetProcAddress(
            module,
            PCSTR(c"SetWindowCompositionAttribute".as_ptr().cast()),
        )?
    };
    Some(unsafe {
        std::mem::transmute::<unsafe extern "system" fn() -> isize, SetWindowCompositionAttributeFn>(
            proc,
        )
    })
}

fn build_policy(mode: TaskbarAppearanceMode, argb: u32) -> AccentPolicy {
    match mode {
        TaskbarAppearanceMode::Normal => AccentPolicy {
            state: AccentState::Disabled as u32,
            flags: 0,
            gradient_abgr: 0,
            animation_id: 0,
        },
        TaskbarAppearanceMode::Opaque => AccentPolicy {
            state: AccentState::Gradient as u32,
            flags: 0x2,
            gradient_abgr: argb_to_abgr(0xFF00_0000 | (argb & 0x00FF_FFFF)),
            animation_id: 0,
        },
        TaskbarAppearanceMode::Clear => AccentPolicy {
            state: AccentState::TransparentGradient as u32,
            flags: 0x2,
            gradient_abgr: argb_to_abgr(argb & 0x00FF_FFFF),
            animation_id: 0,
        },
        TaskbarAppearanceMode::Acrylic => AccentPolicy {
            state: AccentState::AcrylicBlurBehind as u32,
            flags: 0x2,
            gradient_abgr: argb_to_abgr(argb),
            animation_id: 0,
        },
    }
}

pub fn apply_classic_swca(mode: TaskbarAppearanceMode, argb: u32) -> (TaskbarFxResultCode, String) {
    let Some(swca) = load_swca() else {
        return (
            TaskbarFxResultCode::ErrMissingExport,
            "SetWindowCompositionAttribute is not exported. This OS does not support the classic path.".to_string(),
        );
    };

    let trays = TaskbarProbe::find_tray_windows();
    if trays.is_empty() {
        return (
            TaskbarFxResultCode::ErrNoTray,
            "No Shell_TrayWnd. Taskbar is not available.".to_string(),
        );
    }

    let policy = build_policy(mode, argb);
    let mut applied = 0i32;
    let mut failed = 0i32;

    for hwnd in trays {
        if TaskbarProbe::has_xaml_bridge(hwnd) {
            continue;
        }
        let mut local = policy;
        let mut data = WindowCompositionAttributeData {
            attribute: WCA_ACCENT_POLICY,
            data: (&mut local as *mut AccentPolicy).cast(),
            data_size: std::mem::size_of::<AccentPolicy>() as u32,
        };
        let ok = unsafe { swca(hwnd, &mut data).as_bool() };
        if ok {
            applied += 1;
        } else {
            failed += 1;
        }
    }

    if applied == 0 {
        return (
            TaskbarFxResultCode::ErrCallFailed,
            "SetWindowCompositionAttribute failed on every taskbar window.".to_string(),
        );
    }
    if failed > 0 {
        return (
            TaskbarFxResultCode::Ok,
            "Some secondary taskbars could not be updated.".to_string(),
        );
    }
    if mode == TaskbarAppearanceMode::Normal {
        (
            TaskbarFxResultCode::Ok,
            "Restored system default (did not end explorer).".to_string(),
        )
    } else {
        (
            TaskbarFxResultCode::Ok,
            "Applied classic taskbar appearance.".to_string(),
        )
    }
}
