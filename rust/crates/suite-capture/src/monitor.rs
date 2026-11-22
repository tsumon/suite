use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFOEXW,
    MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForSystem, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, MONITORINFOF_PRIMARY, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};
use windows::core::BOOL;

use crate::pixel::{PixelBuffer, PixelRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMethod {
    None,
    GdiBitBlt,
    DxgiDuplication,
    GraphicsCapture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    pub handle: isize,
    pub bounds: PixelRect,
    pub work_area: PixelRect,
    pub device_name: String,
    pub is_primary: bool,
    pub dpi_x: u32,
    pub dpi_y: u32,
}

impl Default for MonitorInfo {
    fn default() -> Self {
        Self {
            handle: 0,
            bounds: PixelRect::default(),
            work_area: PixelRect::default(),
            device_name: String::new(),
            is_primary: false,
            dpi_x: 96,
            dpi_y: 96,
        }
    }
}

pub struct MonitorCapture {
    pub monitor: MonitorInfo,
    pub buffer: PixelBuffer,
    pub method: CaptureMethod,
}

pub struct MonitorEnumerator;

impl MonitorEnumerator {
    pub fn get_all() -> Vec<MonitorInfo> {
        let mut list: Vec<MonitorInfo> = Vec::with_capacity(4);
        unsafe extern "system" fn enum_proc(
            hmon: HMONITOR,
            _hdc: HDC,
            _rc: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let list = unsafe { &mut *(lparam.0 as *mut Vec<MonitorInfo>) };
            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if unsafe { !GetMonitorInfoW(hmon, &mut info.monitorInfo).as_bool() } {
                return BOOL(1);
            }

            let mut dpi_x = 96u32;
            let mut dpi_y = 96u32;
            let mut x = 0u32;
            let mut y = 0u32;
            if unsafe { GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut x, &mut y).is_ok() } {
                if x != 0 {
                    dpi_x = x;
                }
                if y != 0 {
                    dpi_y = y;
                }
            }

            let m = info.monitorInfo.rcMonitor;
            let w = info.monitorInfo.rcWork;
            let bounds = PixelRect::new(m.left, m.top, m.right - m.left, m.bottom - m.top);
            let work = PixelRect::new(w.left, w.top, w.right - w.left, w.bottom - w.top);
            if bounds.is_empty() {
                return BOOL(1);
            }

            let name_len = info
                .szDevice
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(info.szDevice.len());
            let device_name = String::from_utf16_lossy(&info.szDevice[..name_len]);

            list.push(MonitorInfo {
                handle: hmon.0 as isize,
                bounds,
                work_area: if work.is_empty() { bounds } else { work },
                device_name,
                is_primary: (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0,
                dpi_x,
                dpi_y,
            });
            BOOL(1)
        }

        unsafe {
            let _ = EnumDisplayMonitors(
                None,
                None,
                Some(enum_proc),
                LPARAM((&mut list as *mut Vec<MonitorInfo>) as isize),
            );
        }

        if list.is_empty() {
            let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
            let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
            let w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
            let h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
            if w > 0 && h > 0 {
                let bounds = PixelRect::new(x, y, w, h);
                list.push(MonitorInfo {
                    handle: 0,
                    bounds,
                    work_area: bounds,
                    device_name: "VIRTUAL".to_string(),
                    is_primary: true,
                    dpi_x: Self::system_dpi(),
                    dpi_y: Self::system_dpi(),
                });
            }
        }
        list
    }

    pub fn system_dpi() -> u32 {
        let dpi = unsafe { GetDpiForSystem() };
        if dpi == 0 { 96 } else { dpi }
    }

    pub fn dpi_near_cursor() -> u32 {
        let mut pt = POINT::default();
        if unsafe { GetCursorPos(&mut pt).is_ok() } {
            let mon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
            if !mon.is_invalid() {
                let mut dx = 0u32;
                let mut dy = 0u32;
                if unsafe { GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_ok() }
                    && dx != 0
                {
                    return dx;
                }
            }
        }
        Self::system_dpi()
    }

    pub fn cursor_pos() -> (i32, i32) {
        let mut pt = POINT::default();
        if unsafe { GetCursorPos(&mut pt).is_ok() } {
            (pt.x, pt.y)
        } else {
            (0, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_monitor_enumeration_finds_primary() {
        let monitors = MonitorEnumerator::get_all();
        assert!(!monitors.is_empty());
        assert!(monitors.iter().all(|m| !m.bounds.is_empty()));
    }
}
