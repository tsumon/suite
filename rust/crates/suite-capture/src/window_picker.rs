use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::WindowsAndMessaging::{
    CWP_SKIPINVISIBLE, CWP_SKIPTRANSPARENT, ChildWindowFromPointEx, EnumWindows, GA_PARENT,
    GA_ROOT, GetAncestor, GetClassNameW, GetWindowRect, IsWindowVisible,
};
use windows::core::BOOL;

use crate::monitor::MonitorInfo;
use crate::pixel::{CaptureUx, PixelRect};

const DESKTOP_CLASSES: &[&str] = &[
    "Progman",
    "WorkerW",
    "#32769",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
];

pub struct WindowPicker;

impl WindowPicker {
    pub fn hit(
        screen_x: i32,
        screen_y: i32,
        ignore_hwnds: &[isize],
        monitors: &[MonitorInfo],
    ) -> PixelRect {
        let by_enum = Self::hit_by_z_order(screen_x, screen_y, ignore_hwnds);
        if !by_enum.is_empty() {
            return by_enum;
        }
        Self::work_area_at(screen_x, screen_y, monitors)
    }

    fn hit_by_z_order(screen_x: i32, screen_y: i32, ignore_hwnds: &[isize]) -> PixelRect {
        struct SearchCtx<'a> {
            x: i32,
            y: i32,
            ignore: &'a [isize],
            hit: HWND,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            if hwnd.is_invalid() {
                return BOOL(1);
            }
            let ctx = unsafe { &mut *(lparam.0 as *mut SearchCtx<'_>) };
            if WindowPicker::should_ignore(ctx.ignore, hwnd) {
                return BOOL(1);
            }
            if !WindowPicker::is_usable_window(hwnd) {
                return BOOL(1);
            }
            let Some(bounds) = WindowPicker::try_rect(hwnd) else {
                return BOOL(1);
            };
            if bounds.is_empty() || !bounds.contains(ctx.x, ctx.y) {
                return BOOL(1);
            }
            ctx.hit = hwnd;
            BOOL(0)
        }

        let mut ctx = SearchCtx {
            x: screen_x,
            y: screen_y,
            ignore: ignore_hwnds,
            hit: HWND::default(),
        };
        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM((&mut ctx as *mut SearchCtx<'_>) as isize));
        }

        if ctx.hit.is_invalid() {
            return PixelRect::default();
        }

        let deep = Self::deepest_child(ctx.hit, screen_x, screen_y);
        if !deep.is_invalid()
            && deep != ctx.hit
            && Self::is_usable_window(deep)
            && let Some(child) = Self::try_rect(deep)
            && !child.is_empty()
            && child.width >= CaptureUx::MIN_COMMIT_PX
            && child.height >= CaptureUx::MIN_COMMIT_PX
        {
            return child;
        }

        if let Some(top) = Self::try_rect(ctx.hit)
            && !top.is_empty()
            && top.width >= CaptureUx::MIN_COMMIT_PX
            && top.height >= CaptureUx::MIN_COMMIT_PX
        {
            return top;
        }

        PixelRect::default()
    }

    fn deepest_child(root: HWND, screen_x: i32, screen_y: i32) -> HWND {
        let mut current = root;
        for _ in 0..24 {
            let mut pt = POINT {
                x: screen_x,
                y: screen_y,
            };
            if unsafe { !ScreenToClient(current, &mut pt).as_bool() } {
                break;
            }
            let child = unsafe {
                ChildWindowFromPointEx(current, pt, CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT)
            };
            if child.is_invalid() || child == current {
                break;
            }
            current = child;
        }
        current
    }

    fn try_rect(hwnd: HWND) -> Option<PixelRect> {
        let mut rc = RECT::default();
        let got_dwm = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&mut rc as *mut RECT).cast(),
                std::mem::size_of::<RECT>() as u32,
            )
            .is_ok()
        };
        if !got_dwm && unsafe { GetWindowRect(hwnd, &mut rc).is_err() } {
            return None;
        }
        if rc.right <= rc.left || rc.bottom <= rc.top {
            return None;
        }
        Some(PixelRect::new(
            rc.left,
            rc.top,
            rc.right - rc.left,
            rc.bottom - rc.top,
        ))
    }

    fn is_usable_window(hwnd: HWND) -> bool {
        if hwnd.is_invalid() || unsafe { !IsWindowVisible(hwnd).as_bool() } || Self::is_cloaked(hwnd) {
            return false;
        }
        let cls = Self::class_name(hwnd);
        !DESKTOP_CLASSES.iter().any(|&d| cls == d)
    }

    fn is_cloaked(hwnd: HWND) -> bool {
        let mut cloaked: i32 = 0;
        let ok = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                (&mut cloaked as *mut i32).cast(),
                std::mem::size_of::<i32>() as u32,
            )
            .is_ok()
        };
        ok && cloaked != 0
    }

    fn class_name(hwnd: HWND) -> String {
        let mut buf = [0u16; 256];
        let len = unsafe { GetClassNameW(hwnd, &mut buf) };
        if len <= 0 {
            String::new()
        } else {
            String::from_utf16_lossy(&buf[..len as usize])
        }
    }

    pub fn work_area_at(screen_x: i32, screen_y: i32, monitors: &[MonitorInfo]) -> PixelRect {
        for m in monitors {
            if m.bounds.contains(screen_x, screen_y) {
                return if m.work_area.is_empty() {
                    m.bounds
                } else {
                    m.work_area
                };
            }
        }
        monitors
            .first()
            .map(|m| m.bounds)
            .unwrap_or_else(|| PixelRect::new(screen_x, screen_y, 4, 4))
    }

    fn should_ignore(ignore_hwnds: &[isize], hwnd: HWND) -> bool {
        let raw = hwnd.0 as isize;
        let root = unsafe { GetAncestor(hwnd, GA_ROOT) };
        let root_raw = root.0 as isize;
        for &item in ignore_hwnds {
            if item == 0 {
                continue;
            }
            if item == raw || (root_raw != 0 && item == root_raw) {
                return true;
            }
            let mut walk = hwnd;
            for _ in 0..8 {
                if walk.is_invalid() {
                    break;
                }
                if walk.0 as isize == item {
                    return true;
                }
                walk = unsafe { GetAncestor(walk, GA_PARENT) };
            }
        }
        false
    }
}
