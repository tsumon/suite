use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
    GetPixel, ReleaseDC, SelectObject,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, EnumChildWindows, EnumWindows, FindWindowExW, FindWindowW, GetClassNameW,
    GetClientRect, GetLayeredWindowAttributes, GetParent, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST, IsWindow,
    IsWindowVisible, LWA_COLORKEY, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SWP_SHOWWINDOW, SetLayeredWindowAttributes, SetParent, SetWindowLongPtrW,
    SetWindowPos, ULW_ALPHA, UpdateLayeredWindow, WS_CHILD, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};
use windows::core::{BOOL, PCWSTR};

use crate::constants::to_wide_null;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TaskbarKind {
    NotFound = 0,
    ClassicWin32 = 1,
    ModernXaml = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl TrayRect {
    pub fn width(self) -> i32 {
        self.right - self.left
    }

    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
}

pub struct TaskbarProbe;

impl TaskbarProbe {
    pub const PRIMARY_TRAY_CLASS: &'static str = "Shell_TrayWnd";
    pub const SECONDARY_TRAY_CLASS: &'static str = "Shell_SecondaryTrayWnd";
    pub const XAML_BRIDGE_CLASS: &'static str = "Windows.UI.Composition.DesktopWindowContentBridge";

    pub fn detect() -> TaskbarKind {
        let primary = Self::find_primary_tray();
        if primary.is_invalid() || !unsafe { IsWindow(Some(primary)).as_bool() } {
            return TaskbarKind::NotFound;
        }
        if Self::has_xaml_bridge(primary) {
            return TaskbarKind::ModernXaml;
        }
        for tray in Self::find_secondary_trays() {
            if Self::has_xaml_bridge(tray) {
                return TaskbarKind::ModernXaml;
            }
        }
        TaskbarKind::ClassicWin32
    }

    pub fn find_primary_tray() -> HWND {
        let cls = to_wide_null(Self::PRIMARY_TRAY_CLASS);
        unsafe { FindWindowW(PCWSTR(cls.as_ptr()), PCWSTR::null()).unwrap_or_default() }
    }

    pub fn find_secondary_trays() -> Vec<HWND> {
        let mut list: Vec<HWND> = Vec::new();
        unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let list = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
            if get_class_name(hwnd) == TaskbarProbe::SECONDARY_TRAY_CLASS
                && unsafe { IsWindow(Some(hwnd)).as_bool() }
                && !list.contains(&hwnd)
            {
                list.push(hwnd);
            }
            BOOL(1)
        }
        unsafe {
            let _ = EnumWindows(Some(enum_cb), LPARAM((&mut list as *mut Vec<HWND>) as isize));
        }
        list
    }

    pub fn find_tray_windows() -> Vec<HWND> {
        let mut list = Vec::new();
        let primary = Self::find_primary_tray();
        if !primary.is_invalid() && unsafe { IsWindow(Some(primary)).as_bool() } {
            list.push(primary);
        }
        for sec in Self::find_secondary_trays() {
            if !list.contains(&sec) {
                list.push(sec);
            }
        }
        list
    }

    pub fn has_xaml_bridge(tray: HWND) -> bool {
        if tray.is_invalid() {
            return false;
        }
        let cls = to_wide_null(Self::XAML_BRIDGE_CLASS);
        let child = unsafe { FindWindowExW(Some(tray), None, PCWSTR(cls.as_ptr()), PCWSTR::null()) }
            .unwrap_or_default();
        !child.is_invalid() && unsafe { IsWindow(Some(child)).as_bool() }
    }
}

pub fn get_class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

pub fn get_window_text(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

static ORIGINAL_STYLES: OnceLock<Mutex<HashMap<isize, isize>>> = OnceLock::new();

fn original_styles() -> &'static Mutex<HashMap<isize, isize>> {
    ORIGINAL_STYLES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct TaskbarEmbed;

impl TaskbarEmbed {
    pub const DEFAULT_WIDGET_WIDTH_PX: i32 = 148;
    pub const NOTIFY_CLASS: &'static str = "TrayNotifyWnd";
    pub const NET_SPEED_TRAY_TITLE: &'static str = "Suite NetSpeed Tray";
    pub const NET_SPEED_FLOAT_TITLE: &'static str = "Suite NetSpeed";
    pub const FALLBACK_OPAQUE_ARGB: u32 = 0xFF2B_2B2B;

    pub fn find_primary_tray() -> HWND {
        let tray = TaskbarProbe::find_primary_tray();
        if !tray.is_invalid() && unsafe { IsWindow(Some(tray)).as_bool() } {
            tray
        } else {
            HWND::default()
        }
    }

    pub fn compute_slot(
        tray_client_width: i32,
        tray_client_height: i32,
        notify_left: Option<i32>,
        desired_width: i32,
    ) -> (i32, i32, i32, i32) {
        let height = tray_client_height.max(1);
        let max_w = (tray_client_width - 8).max(40);
        let width = desired_width.clamp(40, max_w);
        let mut x = match notify_left {
            Some(left) if left - width - 4 >= 0 => left - width - 4,
            _ => (tray_client_width - width - 4).max(0),
        };
        if x + width > tray_client_width {
            x = (tray_client_width - width).max(0);
        }
        (x, 0, width, height)
    }

    pub fn slot_overlaps_notify(slot_x: i32, slot_width: i32, notify_left: Option<i32>) -> bool {
        match notify_left {
            Some(left) if left >= 0 => slot_x + slot_width > left,
            _ => false,
        }
    }

    pub fn try_get_window_rect(hwnd: HWND) -> Option<TrayRect> {
        if hwnd.is_invalid() {
            return None;
        }
        let mut rc = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut rc).is_ok() } {
            Some(TrayRect {
                left: rc.left,
                top: rc.top,
                right: rc.right,
                bottom: rc.bottom,
            })
        } else {
            None
        }
    }

    pub fn is_window_visible(hwnd: HWND) -> bool {
        !hwnd.is_invalid()
            && unsafe { IsWindow(Some(hwnd)).as_bool() && IsWindowVisible(hwnd).as_bool() }
    }

    pub fn is_attached(hwnd: HWND) -> bool {
        if hwnd.is_invalid() {
            return false;
        }
        let tray = Self::find_primary_tray();
        if tray.is_invalid() {
            return false;
        }
        let parent = unsafe { GetParent(hwnd).unwrap_or_default() };
        if parent != tray {
            return false;
        }
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        (style & (WS_CHILD.0 as isize)) != 0
    }

    pub fn dpi_for(hwnd: HWND) -> u32 {
        if hwnd.is_invalid() {
            return 96;
        }
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        if dpi == 0 { 96 } else { dpi }
    }

    fn find_notify_left(tray: HWND, tray_screen_left: i32) -> Option<i32> {
        let cls = to_wide_null(Self::NOTIFY_CLASS);
        let notify = unsafe { FindWindowExW(Some(tray), None, PCWSTR(cls.as_ptr()), PCWSTR::null()) }
            .unwrap_or_default();
        if notify.is_invalid() {
            return None;
        }
        Self::try_get_window_rect(notify).map(|rc| rc.left - tray_screen_left)
    }

    /// Top-level TopMost layered-capable window positioned IN the tray slot (left of TrayNotifyWnd).
    /// No SetParent — UpdateLayeredWindow works reliably; WS_CHILD of Shell_TrayWnd does not.
    pub fn try_dock_in_tray_slot(hwnd: HWND, tray: HWND, widget_width_px: i32) -> Result<(), String> {
        if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return Err("窗口句柄无效。".to_string());
        }
        let Some(tray_rc) = (!tray.is_invalid() && unsafe { IsWindow(Some(tray)).as_bool() })
            .then(|| Self::try_get_window_rect(tray))
            .flatten()
        else {
            return Err("找不到任务栏。".to_string());
        };

        if Self::is_in_tray_slot(hwnd, tray, widget_width_px, 4) {
            return Ok(());
        }

        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        let parent = unsafe { GetParent(hwnd).unwrap_or_default() };
        if (style & (WS_CHILD.0 as isize)) != 0 || parent == tray {
            unsafe {
                let _ = SetParent(hwnd, None);
            }
            let restored = {
                let guard = original_styles().lock().unwrap_or_else(|e| e.into_inner());
                guard.get(&(hwnd.0 as isize)).copied()
            };
            let next_style = restored.unwrap_or({
                (style | (WS_POPUP.0 as isize) | (WS_VISIBLE.0 as isize)) & !(WS_CHILD.0 as isize)
            });
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, next_style);
            }
        }

        let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        let desired_ex = (ex
            | (WS_EX_TOPMOST.0 as isize)
            | (WS_EX_TOOLWINDOW.0 as isize)
            | (WS_EX_LAYERED.0 as isize))
            & !(WS_EX_TRANSPARENT.0 as isize);
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired_ex);
        }

        let tray_w = tray_rc.width();
        let tray_h = tray_rc.height();
        let notify_left = Self::find_notify_left(tray, tray_rc.left);
        let slot = Self::compute_slot(tray_w, tray_h, notify_left, widget_width_px);
        let screen_x = tray_rc.left + slot.0;
        let screen_y = tray_rc.top;

        let ok = unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                screen_x,
                screen_y,
                slot.2,
                slot.3,
                SWP_SHOWWINDOW | SWP_NOACTIVATE,
            )
        };
        if ok.is_err() {
            return Err("任务栏里没有足够位置。".to_string());
        }

        let Some(rc) = Self::try_get_window_rect(hwnd) else {
            return Err("停靠任务栏后仍不可见。".to_string());
        };
        if rc.width() < 8 || rc.height() < 8 || !unsafe { IsWindowVisible(hwnd).as_bool() } {
            return Err("停靠任务栏后仍不可见。".to_string());
        }
        Ok(())
    }

    pub fn try_dock_in_primary_tray_slot(hwnd: HWND, widget_width_px: i32) -> Result<(), String> {
        Self::try_dock_in_tray_slot(hwnd, Self::find_primary_tray(), widget_width_px)
    }

    pub fn is_in_tray_slot(hwnd: HWND, tray: HWND, widget_width_px: i32, slack_px: i32) -> bool {
        if !Self::is_window_visible(hwnd) {
            return false;
        }
        let Some(tray_rc) = (!tray.is_invalid() && unsafe { IsWindow(Some(tray)).as_bool() })
            .then(|| Self::try_get_window_rect(tray))
            .flatten()
        else {
            return false;
        };
        if Self::is_attached(hwnd) || unsafe { GetParent(hwnd).unwrap_or_default() } == tray {
            return false;
        }
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        if (style & (WS_CHILD.0 as isize)) != 0 {
            return false;
        }

        let notify_left = Self::find_notify_left(tray, tray_rc.left);
        let slot = Self::compute_slot(tray_rc.width(), tray_rc.height(), notify_left, widget_width_px);
        let expect_x = tray_rc.left + slot.0;
        let expect_y = tray_rc.top;
        let Some(rc) = Self::try_get_window_rect(hwnd) else {
            return false;
        };
        if rc.width() < 8 || rc.height() < 8 {
            return false;
        }
        (rc.left - expect_x).abs() <= slack_px
            && (rc.top - expect_y).abs() <= slack_px
            && (rc.width() - slot.2).abs() <= slack_px + 2
            && (rc.height() - slot.3).abs() <= slack_px + 2
    }

    pub fn is_docked_over_primary_tray(hwnd: HWND) -> bool {
        if !Self::is_window_visible(hwnd) || Self::is_attached(hwnd) {
            return false;
        }
        let Some(rc) = Self::try_get_window_rect(hwnd) else {
            return false;
        };
        if rc.width() < 8 || rc.height() < 8 {
            return false;
        }
        let tray = Self::find_primary_tray();
        let Some(tr) = Self::try_get_window_rect(tray) else {
            return false;
        };
        let horiz = rc.left < tr.right && rc.right > tr.left;
        let vert = rc.bottom >= tr.top - 4 && rc.top <= tr.bottom + 4;
        horiz && vert
    }

    pub fn is_proven_visible_on_primary_tray(hwnd: HWND) -> bool {
        Self::is_docked_over_primary_tray(hwnd)
    }

    pub fn try_sample_tray_argb() -> Option<u32> {
        let tray = Self::find_primary_tray();
        let tr = Self::try_get_window_rect(tray)?;
        let tray_h = tr.height().max(1);
        let tray_w = tr.width().max(1);
        let mut sample_x = tr.left + (tray_w / 3).max(8);
        let cls = to_wide_null(Self::NOTIFY_CLASS);
        let notify = unsafe { FindWindowExW(Some(tray), None, PCWSTR(cls.as_ptr()), PCWSTR::null()) }
            .unwrap_or_default();
        if let Some(nrc) = Self::try_get_window_rect(notify) {
            sample_x = (nrc.left - 24).max(tr.left + 4);
        }
        let sample_y = tr.top + tray_h / 2;

        unsafe {
            let hdc = GetDC(None);
            if hdc.is_invalid() {
                return None;
            }
            let color_ref: COLORREF = GetPixel(hdc, sample_x, sample_y);
            let _ = ReleaseDC(None, hdc);
            if color_ref.0 == 0xFFFF_FFFF {
                return None;
            }
            let r = (color_ref.0 & 0xFF) as u8;
            let g = ((color_ref.0 >> 8) & 0xFF) as u8;
            let b = ((color_ref.0 >> 16) & 0xFF) as u8;
            if r == 0xFF && g == 0x00 && b == 0xFF {
                return None;
            }
            Some(0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
        }
    }

    pub fn prepare_per_pixel_layered(hwnd: HWND) {
        if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return;
        }
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                ex & !(WS_EX_LAYERED.0 as isize) & !(WS_EX_TRANSPARENT.0 as isize),
            );
            let ex2 = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex2 | (WS_EX_LAYERED.0 as isize));
        }
    }

    pub fn clear_layered_transparency(hwnd: HWND) {
        if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return;
        }
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if (ex & (WS_EX_LAYERED.0 as isize)) != 0 {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex & !(WS_EX_LAYERED.0 as isize));
            }
        }
    }

    /// Push a full 32bpp premultiplied-BGRA frame via `UpdateLayeredWindow(ULW_ALPHA)`.
    pub fn try_push_per_pixel_layered(
        hwnd: HWND,
        width: i32,
        height: i32,
        bgra_premul: &[u8],
    ) -> bool {
        if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return false;
        }
        if width < 1 || height < 1 || bgra_premul.len() < (width * height * 4) as usize {
            return false;
        }

        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let mut needs_prep = (ex & (WS_EX_LAYERED.0 as isize)) == 0;
            if !needs_prep {
                let mut flags = Default::default();
                if GetLayeredWindowAttributes(hwnd, None, None, Some(&mut flags)).is_ok()
                    && (flags & LWA_COLORKEY).0 != 0
                {
                    needs_prep = true;
                }
            }
            if needs_prep {
                Self::prepare_per_pixel_layered(hwnd);
            }

            let screen_dc = GetDC(None);
            if screen_dc.is_invalid() {
                return false;
            }
            let mem_dc = CreateCompatibleDC(Some(screen_dc));
            if mem_dc.is_invalid() {
                let _ = ReleaseDC(None, screen_dc);
                return false;
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(Some(mem_dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
            let Ok(dib_bmp) = dib else {
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(None, screen_dc);
                return false;
            };
            if dib_bmp.is_invalid() || bits.is_null() {
                if !dib_bmp.is_invalid() {
                    let _ = DeleteObject(dib_bmp.into());
                }
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(None, screen_dc);
                return false;
            }

            let old = SelectObject(mem_dc, dib_bmp.into());
            std::ptr::copy_nonoverlapping(
                bgra_premul.as_ptr(),
                bits.cast::<u8>(),
                (width * height * 4) as usize,
            );

            let size = SIZE {
                cx: width,
                cy: height,
            };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let ok = UpdateLayeredWindow(
                hwnd,
                None,
                None,
                Some(&size),
                Some(mem_dc),
                Some(&src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            SelectObject(mem_dc, old);
            let _ = DeleteObject(dib_bmp.into());
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            ok.is_ok()
        }
    }

    pub fn is_netspeed_window_title(title: &str) -> bool {
        title == Self::NET_SPEED_TRAY_TITLE
            || title == Self::NET_SPEED_FLOAT_TITLE
            || title.starts_with("Suite NetSpeed")
    }

    pub fn destroy_hwnd(hwnd: HWND) {
        if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return;
        }
        Self::clear_layered_transparency(hwnd);
        unsafe {
            let _ = SetParent(hwnd, None);
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let next = (style | (WS_POPUP.0 as isize)) & !(WS_CHILD.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_STYLE, next);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOSIZE | SWP_NOMOVE,
            );
            let _ = DestroyWindow(hwnd);
        }
    }

    pub fn cleanup_orphan_netspeed_windows(keep_pid: Option<u32>) -> usize {
        struct EnumCtx {
            keep_pid: Option<u32>,
            doomed: Vec<HWND>,
        }
        unsafe extern "system" fn child_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            if hwnd.is_invalid() || !unsafe { IsWindow(Some(hwnd)).as_bool() } {
                return BOOL(1);
            }
            let ctx = unsafe { &mut *(lparam.0 as *mut EnumCtx) };
            let mut pid = 0u32;
            unsafe {
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
            }
            if ctx.keep_pid == Some(pid) {
                return BOOL(1);
            }
            let title = get_window_text(hwnd);
            if TaskbarEmbed::is_netspeed_window_title(&title) {
                ctx.doomed.push(hwnd);
            }
            BOOL(1)
        }

        let mut destroyed = 0usize;
        for tray in TaskbarProbe::find_tray_windows() {
            if tray.is_invalid() {
                continue;
            }
            let mut ctx = EnumCtx {
                keep_pid,
                doomed: Vec::new(),
            };
            unsafe {
                let _ = EnumChildWindows(
                    Some(tray),
                    Some(child_cb),
                    LPARAM((&mut ctx as *mut EnumCtx) as isize),
                );
            }
            for h in ctx.doomed {
                Self::destroy_hwnd(h);
                destroyed += 1;
            }
        }
        destroyed
    }

    pub fn try_get_tray_client_size() -> Option<(i32, i32)> {
        let tray = Self::find_primary_tray();
        if tray.is_invalid() {
            return None;
        }
        let mut rc = RECT::default();
        if unsafe { GetClientRect(tray, &mut rc).is_err() } {
            return None;
        }
        let w = rc.right - rc.left;
        let h = rc.bottom - rc.top;
        if w >= 32 && h >= 16 {
            Some((w, h))
        } else {
            None
        }
    }

    pub fn set_click_through(hwnd: HWND, enabled: bool) {
        if hwnd.is_invalid() {
            return;
        }
        unsafe {
            let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            if enabled {
                ex |= (WS_EX_TRANSPARENT.0 as isize) | (WS_EX_LAYERED.0 as isize);
            } else {
                ex &= !(WS_EX_TRANSPARENT.0 as isize);
            }
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
    }
}

#[allow(dead_code)]
fn _unused_colorkey_keep() {
    let _ = SetLayeredWindowAttributes;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_slot_left_of_notify() {
        let (x, y, w, h) = TaskbarEmbed::compute_slot(1920, 48, Some(1700), 148);
        assert_eq!(x, 1700 - 148 - 4);
        assert_eq!(y, 0);
        assert_eq!(w, 148);
        assert_eq!(h, 48);
        assert!(!TaskbarEmbed::slot_overlaps_notify(x, w, Some(1700)));
    }

    #[test]
    fn compute_slot_falls_back_when_notify_missing() {
        let (x, _, w, _) = TaskbarEmbed::compute_slot(1920, 48, None, 148);
        assert_eq!(x, 1920 - 148 - 4);
        assert_eq!(w, 148);
    }

    #[test]
    fn probe_runs_without_panic() {
        let kind = TaskbarProbe::detect();
        assert!(matches!(
            kind,
            TaskbarKind::NotFound | TaskbarKind::ClassicWin32 | TaskbarKind::ModernXaml
        ));
    }
}
