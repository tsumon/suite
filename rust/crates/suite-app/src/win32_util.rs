use std::path::{Path, PathBuf};

use suite_capture::MonitorEnumerator;
use suite_platform::constants::{to_wide_null, wide_ptr_to_string};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, CreatePen, CreateSolidBrush,
    DEFAULT_CHARSET, DEFAULT_PITCH, DeleteObject, FF_DONTCARE, FillRect, FW_BOLD, FW_NORMAL,
    FW_SEMIBOLD, GetMonitorInfoW, GetStockObject, HDC, HFONT, InvalidateRect,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, NULL_PEN, OUT_DEFAULT_PRECIS,
    PS_SOLID, RoundRect, SelectObject,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::UI::Controls::Dialogs::{
    CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, ChooseColorW, GetSaveFileNameW, OFN_OVERWRITEPROMPT,
    OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Controls::SetWindowTheme;
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog,
    IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, GetCursorPos, HCURSOR, IDC_ARROW, IDOK, LoadCursorW,
    MB_ICONWARNING, MB_OKCANCEL, MessageBoxW, RegisterClassExW, SW_SHOWNORMAL, WM_SETFONT,
    WNDCLASSEXW, WNDPROC,
};
use windows::core::{BOOL, PCWSTR};

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

pub fn dpi_for_cursor() -> u32 {
    MonitorEnumerator::dpi_near_cursor().max(96)
}

pub fn s_px(dip: i32, dpi: u32) -> i32 {
    ((dip as f64) * (dpi.max(96) as f64) / 96.0).round() as i32
}

pub fn centered_origin_on_cursor_monitor(w_px: i32, h_px: i32) -> (i32, i32) {
    let mut pt = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    let hmon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(hmon, &mut mi).as_bool() } {
        let work = mi.rcWork;
        let ww = (work.right - work.left).max(1);
        let wh = (work.bottom - work.top).max(1);
        let x = work.left + ((ww - w_px) / 2).max(0);
        let y = work.top + ((wh - h_px) / 2).max(0);
        (x, y)
    } else {
        (160, 120)
    }
}

pub fn apply_window_theme(hwnd: HWND, dark: bool) {
    if hwnd.is_invalid() {
        return;
    }
    let dark_val = BOOL::from(dark);
    let round_pref = DWMWCP_ROUND;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark_val as *const BOOL).cast(),
            std::mem::size_of::<BOOL>() as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&round_pref as *const _) as *const _,
            std::mem::size_of_val(&round_pref) as u32,
        );
    }
}

pub fn apply_control_theme(hwnd: HWND, dark: bool) {
    if hwnd.is_invalid() {
        return;
    }
    let sub = to_wide_null(if dark { "DarkMode_Explorer" } else { "Explorer" });
    unsafe {
        let _ = SetWindowTheme(hwnd, PCWSTR(sub.as_ptr()), PCWSTR::null());
    }
}

pub fn pick_folder_dialog(owner: HWND, initial_dir: &str) -> Option<PathBuf> {
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        let opts = dialog.GetOptions().unwrap_or_default();
        let _ = dialog.SetOptions(opts | FOS_PICKFOLDERS | FOS_PATHMUSTEXIST | FOS_FORCEFILESYSTEM);

        let trimmed = initial_dir.trim();
        if !trimmed.is_empty() && Path::new(trimmed).is_dir() {
            let wdir = to_wide_null(trimmed);
            if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(PCWSTR(wdir.as_ptr()), None) {
                let _ = dialog.SetFolder(&item);
            }
        }

        if dialog.Show(Some(owner)).is_err() {
            return None;
        }
        let item = dialog.GetResult().ok()?;
        let pwstr = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path_str = wide_ptr_to_string(PCWSTR(pwstr.as_ptr()));
        CoTaskMemFree(Some(pwstr.as_ptr().cast()));
        if path_str.is_empty() {
            None
        } else {
            Some(PathBuf::from(path_str))
        }
    }
}

pub fn draw_rounded_rect(hdc: HDC, rc: RECT, radius: i32, fill: COLORREF, border: Option<COLORREF>) {
    unsafe {
        let brush = CreateSolidBrush(fill);
        let old_b = SelectObject(hdc, brush.into());
        let (pen, old_p) = if let Some(bc) = border {
            let p = CreatePen(PS_SOLID, 1, bc);
            let op = SelectObject(hdc, p.into());
            (Some(p), op)
        } else {
            let op = SelectObject(hdc, GetStockObject(NULL_PEN));
            (None, op)
        };
        let dia = (radius * 2).max(2);
        let _ = RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, dia, dia);
        SelectObject(hdc, old_p);
        SelectObject(hdc, old_b);
        if let Some(p) = pen {
            let _ = DeleteObject(p.into());
        }
        let _ = DeleteObject(brush.into());
    }
}

pub fn draw_filled_rect(hdc: HDC, rc: RECT, fill: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(fill);
        FillRect(hdc, &rc, brush);
        let _ = DeleteObject(brush.into());
    }
}

pub fn current_working_set_mb() -> f64 {
    let mut mem = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    unsafe {
        if GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut mem,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
        .is_ok()
        {
            (mem.WorkingSetSize as f64) / 1_048_576.0
        } else {
            0.0
        }
    }
}
