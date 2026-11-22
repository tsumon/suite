use std::path::PathBuf;

use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
    FF_DONTCARE, FW_BOLD, FW_NORMAL, FW_SEMIBOLD, HFONT, InvalidateRect, OUT_DEFAULT_PRECIS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::Dialogs::{
    CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, ChooseColorW, GetSaveFileNameW, OFN_OVERWRITEPROMPT,
    OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, HCURSOR, IDC_ARROW, LoadCursorW, MB_ICONWARNING,
    MB_OKCANCEL, IDOK, MessageBoxW, RegisterClassExW, SW_SHOWNORMAL, WM_SETFONT, WNDCLASSEXW,
    WNDPROC,
};
use windows::core::PCWSTR;

pub fn hinstance() -> HINSTANCE {
    unsafe { GetModuleHandleW(None).unwrap_or_default().into() }
}

pub fn register_window_class(
    class_name: &str,
    wnd_proc: WNDPROC,
    extra_dblclks: bool,
    hcursor: Option<HCURSOR>,
) {
    let cls = to_wide_null(class_name);
    let inst = hinstance();
    let cursor = hcursor.unwrap_or_else(|| unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() });
    let mut style = CS_HREDRAW | CS_VREDRAW;
    if extra_dblclks {
        style |= CS_DBLCLKS;
    }
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style,
        lpfnWndProc: wnd_proc,
        hInstance: inst,
        hCursor: cursor,
        lpszClassName: PCWSTR(cls.as_ptr()),
        ..Default::default()
    };
    unsafe {
        let _ = RegisterClassExW(&wc);
    }
}

pub fn create_ui_font(height_px: i32, bold: bool, semibold: bool, family: &str) -> HFONT {
    let wfam = to_wide_null(family);
    let weight = if bold {
        FW_BOLD.0
    } else if semibold {
        FW_SEMIBOLD.0
    } else {
        FW_NORMAL.0
    };
    unsafe {
        CreateFontW(
            -height_px,
            0,
            0,
            0,
            weight as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            PCWSTR(wfam.as_ptr()),
        )
    }
}

pub fn set_control_font(hwnd: HWND, font: HFONT) {
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
}

pub fn argb_to_colorref(argb: u32) -> COLORREF {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    COLORREF((b << 16) | (g << 8) | r)
}

pub fn colorref_to_argb(cr: COLORREF, alpha: u8) -> u32 {
    let r = cr.0 & 0xFF;
    let g = (cr.0 >> 8) & 0xFF;
    let b = (cr.0 >> 16) & 0xFF;
    (u32::from(alpha) << 24) | (r << 16) | (g << 8) | b
}

pub fn choose_color_dialog(owner: HWND, current_argb: u32) -> Option<u32> {
    let mut custom = [0x00FF_FFFFu32; 16].map(COLORREF);
    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: owner,
        rgbResult: argb_to_colorref(current_argb),
        lpCustColors: custom.as_mut_ptr(),
        Flags: CC_FULLOPEN | CC_RGBINIT,
        ..Default::default()
    };
    let ok = unsafe { ChooseColorW(&mut cc).as_bool() };
    if ok {
        Some(colorref_to_argb(cc.rgbResult, 0xFF))
    } else {
        None
    }
}

pub fn save_png_dialog(owner: HWND, default_dir: &str, default_name: &str) -> Option<PathBuf> {
    let mut file_buf = vec![0u16; 1024];
    let name_utf16: Vec<u16> = default_name.encode_utf16().collect();
    let copy_len = name_utf16.len().min(file_buf.len() - 1);
    file_buf[..copy_len].copy_from_slice(&name_utf16[..copy_len]);

    let filter: Vec<u16> = "PNG 图片 (*.png)\0*.png\0所有文件 (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let def_ext = to_wide_null("png");
    let init_dir = to_wide_null(default_dir);

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: windows::core::PWSTR(file_buf.as_mut_ptr()),
        nMaxFile: file_buf.len() as u32,
        lpstrInitialDir: if default_dir.trim().is_empty() {
            PCWSTR::null()
        } else {
            PCWSTR(init_dir.as_ptr())
        },
        lpstrDefExt: PCWSTR(def_ext.as_ptr()),
        Flags: OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST,
        ..Default::default()
    };

    let ok = unsafe { GetSaveFileNameW(&mut ofn).as_bool() };
    if !ok {
        return None;
    }
    let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
    Some(PathBuf::from(String::from_utf16_lossy(&file_buf[..len])))
}

pub fn open_shell_target(target: &str) -> bool {
    let verb = to_wide_null("open");
    let wtarget = to_wide_null(target);
    let h = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            PCWSTR(verb.as_ptr()),
            PCWSTR(wtarget.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    (h.0 as usize) > 32
}

pub fn confirm_warning(owner: HWND, title: &str, text: &str) -> bool {
    let wtitle = to_wide_null(title);
    let wtext = to_wide_null(text);
    unsafe {
        MessageBoxW(
            Some(owner),
            PCWSTR(wtext.as_ptr()),
            PCWSTR(wtitle.as_ptr()),
            MB_OKCANCEL | MB_ICONWARNING,
        ) == IDOK
    }
}

pub fn invalidate_all(hwnd: HWND) {
    if !hwnd.is_invalid() {
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, true);
        }
    }
}

pub fn client_rect(hwnd: HWND) -> RECT {
    let mut rc = RECT::default();
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rc);
    }
    rc
}
