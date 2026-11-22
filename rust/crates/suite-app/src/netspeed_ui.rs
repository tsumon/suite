use std::cell::RefCell;
use std::rc::Rc;

use suite_contracts::{NetSpeedSettings, RateFormatter, ThemeKind};
use suite_netspeed::RateSample;
use suite_platform::TaskbarEmbed;
use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, GetCursorPos,
    GetWindowLongPtrW, GWLP_USERDATA, HTCAPTION, KillTimer, PostMessageW,
    SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SetForegroundWindow,
    SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_CONTEXTMENU, WM_DESTROY,
    WM_EXITSIZEMOVE, WM_LBUTTONDOWN, WM_NCDESTROY, WM_NCLBUTTONDOWN, WM_RBUTTONUP, WM_TIMER,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::PCWSTR;

use crate::d2d::D2dRenderer;
use crate::messages::{AppCommand, NetSpeedPosMsg, WM_APP_NETSPEED_POS, post_boxed, post_command};
use crate::win32_util::{hinstance, register_window_class};

const TRAY_WIDGET_CLASS: &str = "SuiteNetSpeedTrayWnd";
const FLOAT_WIDGET_CLASS: &str = "SuiteNetSpeedFloatWnd";
const TIMER_HEARTBEAT: usize = 1;

struct TrayWidgetInner {
    hwnd: HWND,
    controller: HWND,
    d2d: Rc<RefCell<D2dRenderer>>,
    target_tray: HWND,
    appearance: NetSpeedSettings,
    theme: ThemeKind,
    down_bps: f64,
    up_bps: f64,
    adapter_missing: bool,
    embed_requested: bool,
    last_frame_key: String,
    visible: bool,
}

pub struct NetSpeedTrayWidget {
    inner: Rc<RefCell<TrayWidgetInner>>,
}

impl NetSpeedTrayWidget {
    pub fn new(controller: HWND, d2d: Rc<RefCell<D2dRenderer>>, target_tray: HWND) -> Self {
        register_window_class(TRAY_WIDGET_CLASS, Some(tray_wnd_proc), false, None);
        let inner = Rc::new(RefCell::new(TrayWidgetInner {
            hwnd: HWND::default(),
            controller,
            d2d,
            target_tray,
            appearance: NetSpeedSettings::default(),
            theme: ThemeKind::Light,
            down_bps: 0.0,
            up_bps: 0.0,
            adapter_missing: false,
            embed_requested: false,
            last_frame_key: String::new(),
            visible: false,
        }));
        Self { inner }
    }

    fn ensure_window(&self) -> HWND {
        let existing = self.inner.borrow().hwnd;
        if !existing.is_invalid() {
            return existing;
        }
        let cls = to_wide_null(TRAY_WIDGET_CLASS);
        let title = to_wide_null(TaskbarEmbed::NET_SPEED_TRAY_TITLE);
        let raw_ptr = Rc::into_raw(Rc::clone(&self.inner));
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                PCWSTR(cls.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_POPUP,
                0,
                0,
                TaskbarEmbed::DEFAULT_WIDGET_WIDTH_PX,
                40,
                None,
                None,
                Some(hinstance()),
                Some(raw_ptr.cast()),
            )
            .unwrap_or_default()
        };
        if hwnd.is_invalid() {
            unsafe {
                drop(Rc::from_raw(raw_ptr));
            }
            return hwnd;
        }
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
        }
        self.inner.borrow_mut().hwnd = hwnd;
        TaskbarEmbed::prepare_per_pixel_layered(hwnd);
        hwnd
    }

    pub fn apply_appearance(&self, settings: &NetSpeedSettings) {
        let mut s = settings.clone();
        s.normalize();
        let should_push = {
            let mut st = self.inner.borrow_mut();
            st.appearance = s;
            st.last_frame_key.clear();
            st.visible && !st.hwnd.is_invalid()
        };
        if should_push {
            self.push_frame();
        }
    }

    pub fn apply_theme(&self, theme: ThemeKind) {
        let should_push = {
            let mut st = self.inner.borrow_mut();
            st.theme = theme;
            st.last_frame_key.clear();
            st.visible && !st.hwnd.is_invalid()
        };
        if should_push {
            self.push_frame();
        }
    }

    pub fn apply_sample(&self, sample: &RateSample) {
        let should_push = {
            let mut st = self.inner.borrow_mut();
            st.adapter_missing = sample.adapter_missing;
            if sample.adapter_missing {
                st.down_bps = 0.0;
                st.up_bps = 0.0;
            } else {
                st.down_bps = sample.receive_bytes_per_second;
                st.up_bps = sample.send_bytes_per_second;
            }
            st.visible && !st.hwnd.is_invalid()
        };
        if should_push {
            self.push_frame();
        }
    }

    pub fn try_embed(&self, target_tray: Option<HWND>) -> Result<(), String> {
        let hwnd = self.ensure_window();
        if hwnd.is_invalid() {
            return Err("无法创建任务栏网速窗口。".to_string());
        }
        let tray = {
            let mut st = self.inner.borrow_mut();
            if let Some(t) = target_tray {
                st.target_tray = t;
            }
            st.embed_requested = true;
            st.visible = true;
            st.last_frame_key.clear();
            if st.target_tray.is_invalid() {
                TaskbarEmbed::find_primary_tray()
            } else {
                st.target_tray
            }
        };

        let _ = TaskbarEmbed::cleanup_orphan_netspeed_windows(Some(std::process::id()));
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        TaskbarEmbed::try_dock_in_tray_slot(hwnd, tray, TaskbarEmbed::DEFAULT_WIDGET_WIDTH_PX)?;
        self.push_frame();
        unsafe {
            let _ = SetTimer(Some(hwnd), TIMER_HEARTBEAT, 2000, None);
        }
        if !TaskbarEmbed::is_window_visible(hwnd) {
            return Err("嵌入后窗口仍不可见。".to_string());
        }
        Ok(())
    }

    pub fn hide_embed(&self) {
        let hwnd = {
            let mut st = self.inner.borrow_mut();
            st.embed_requested = false;
            st.visible = false;
            st.hwnd
        };
        if !hwnd.is_invalid() {
            unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_HEARTBEAT);
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }

    pub fn destroy(&self) {
        self.hide_embed();
        let hwnd = {
            let mut st = self.inner.borrow_mut();
            let h = st.hwnd;
            st.hwnd = HWND::default();
            h
        };
        if !hwnd.is_invalid() {
            TaskbarEmbed::destroy_hwnd(hwnd);
        }
    }

    pub fn push_frame(&self) {
        let (hwnd, d2d, appearance, theme, down_bps, up_bps, missing, last_key) = {
            let st = self.inner.borrow();
            if st.hwnd.is_invalid() || !st.visible {
                return;
            }
            (
                st.hwnd,
                Rc::clone(&st.d2d),
                st.appearance.clone(),
                st.theme,
                st.down_bps,
                st.up_bps,
                st.adapter_missing,
                st.last_frame_key.clone(),
            )
        };

        let (w, h) = match TaskbarEmbed::try_get_window_rect(hwnd) {
            Some(rc) => (rc.width().max(1), rc.height().max(1)),
            None => (TaskbarEmbed::DEFAULT_WIDGET_WIDTH_PX, 40),
        };
        let key = format!(
            "{w}x{h}|{missing}|{:.0}|{:.0}|{}|{}|{}|{:?}|{}|{:?}|{:?}|{}|{:?}|{}",
            down_bps,
            up_bps,
            appearance.show_download,
            appearance.show_upload,
            appearance.two_line,
            theme,
            appearance.background_transparent,
            appearance.background_argb,
            appearance.down_color_argb,
            appearance.bold,
            appearance.up_color_argb,
            appearance.font_size_dip,
        );
        if key == last_key {
            return;
        }

        let dpi = TaskbarEmbed::dpi_for(hwnd);
        let tray_dark = is_tray_likely_dark(theme);
        let ok = {
            let mut renderer = d2d.borrow_mut();
            if let Some(bytes) = renderer.render_netspeed_frame(
                w, h, dpi, tray_dark, theme, &appearance, down_bps, up_bps, missing,
            ) {
                TaskbarEmbed::try_push_per_pixel_layered(hwnd, w, h, bytes)
            } else {
                false
            }
        };

        if ok {
            self.inner.borrow_mut().last_frame_key = key;
        }
    }
}

impl Drop for NetSpeedTrayWidget {
    fn drop(&mut self) {
        self.destroy();
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
        if raw != 0 {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Rc::from_raw(raw as *const RefCell<TrayWidgetInner>));
            }
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let inner = unsafe { &*(ptr as *const RefCell<TrayWidgetInner>) };

    match msg {
        WM_TIMER if wparam.0 == TIMER_HEARTBEAT => {
            let (embed_req, tray, controller) = {
                let st = inner.borrow();
                let t = if st.target_tray.is_invalid() {
                    TaskbarEmbed::find_primary_tray()
                } else {
                    st.target_tray
                };
                (st.embed_requested, t, st.controller)
            };
            if embed_req && !TaskbarEmbed::is_in_tray_slot(hwnd, tray, TaskbarEmbed::DEFAULT_WIDGET_WIDTH_PX, 4)
                && TaskbarEmbed::try_dock_in_tray_slot(hwnd, tray, TaskbarEmbed::DEFAULT_WIDGET_WIDTH_PX).is_ok() {
                    inner.borrow_mut().last_frame_key.clear();
                    // Push frame on re-dock
                    let (d2d, app, theme, down, up, miss) = {
                        let st = inner.borrow();
                        (
                            Rc::clone(&st.d2d),
                            st.appearance.clone(),
                            st.theme,
                            st.down_bps,
                            st.up_bps,
                            st.adapter_missing,
                        )
                    };
                    if let Some(rc) = TaskbarEmbed::try_get_window_rect(hwnd) {
                        let dpi = TaskbarEmbed::dpi_for(hwnd);
                        let tray_dark = is_tray_likely_dark(theme);
                        let mut r = d2d.borrow_mut();
                        if let Some(bytes) = r.render_netspeed_frame(
                            rc.width().max(1),
                            rc.height().max(1),
                            dpi,
                            tray_dark,
                            theme,
                            &app,
                            down,
                            up,
                            miss,
                        ) {
                            let _ = TaskbarEmbed::try_push_per_pixel_layered(
                                hwnd,
                                rc.width().max(1),
                                rc.height().max(1),
                                bytes,
                            );
                        }
                    }
                    let _ = controller;
                }
            LRESULT(0)
        }
        WM_RBUTTONUP | WM_CONTEXTMENU => {
            let controller = inner.borrow().controller;
            show_tray_chip_menu(hwnd, controller, true);
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_HEARTBEAT);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn is_tray_likely_dark(theme: ThemeKind) -> bool {
    if let Some(argb) = TaskbarEmbed::try_sample_tray_argb() {
        let r = ((argb >> 16) & 0xFF) as i32;
        let g = ((argb >> 8) & 0xFF) as i32;
        let b = (argb & 0xFF) as i32;
        let y = (r * 30 + g * 59 + b * 11) / 100;
        y < 140
    } else {
        theme == ThemeKind::Dark
    }
}

fn show_tray_chip_menu(hwnd: HWND, controller: HWND, is_embedded: bool) {
    unsafe {
        let Ok(hmenu) = CreatePopupMenu() else {
            return;
        };
        let add = |id: usize, text: &str| {
            let w = to_wide_null(text);
            let _ = AppendMenuW(
                hmenu,
                windows::Win32::UI::WindowsAndMessaging::MF_STRING,
                id,
                PCWSTR(w.as_ptr()),
            );
        };
        if is_embedded {
            add(AppCommand::RequestFloatNetSpeed as usize, "改回桌面悬浮窗");
        } else {
            add(AppCommand::RequestEmbedNetSpeed as usize, "嵌入任务栏");
        }
        add(AppCommand::OpenSettings as usize, "设置...");
        add(AppCommand::ToggleNetSpeed as usize, "隐藏网速");

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let cmd_raw = TrackPopupMenu(
            hmenu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x,
            pt.y,
            Some(0),
            hwnd,
            None,
        );
        let _ = DestroyMenu(hmenu);
        if cmd_raw.0 > 0
            && let Some(cmd) = AppCommand::from_wparam(WPARAM(cmd_raw.0 as usize))
        {
            post_command(controller, cmd, 0);
        }
    }
}

struct FloatWidgetInner {
    hwnd: HWND,
    controller: HWND,
    d2d: Rc<RefCell<D2dRenderer>>,
    appearance: NetSpeedSettings,
    theme: ThemeKind,
    down_bps: f64,
    up_bps: f64,
    adapter_missing: bool,
    left_dip: f64,
    top_dip: f64,
    visible: bool,
}

pub struct NetSpeedFloatWindow {
    inner: Rc<RefCell<FloatWidgetInner>>,
}

impl NetSpeedFloatWindow {
    const FLOAT_WIDTH_DIP: f64 = 168.0;
    const FLOAT_HEIGHT_DIP: f64 = 56.0;

    pub fn new(controller: HWND, d2d: Rc<RefCell<D2dRenderer>>) -> Self {
        register_window_class(FLOAT_WIDGET_CLASS, Some(float_wnd_proc), false, None);
        let inner = Rc::new(RefCell::new(FloatWidgetInner {
            hwnd: HWND::default(),
            controller,
            d2d,
            appearance: NetSpeedSettings::default(),
            theme: ThemeKind::Light,
            down_bps: 0.0,
            up_bps: 0.0,
            adapter_missing: false,
            left_dip: 64.0,
            top_dip: 64.0,
            visible: false,
        }));
        Self { inner }
    }

    fn ensure_window(&self) -> HWND {
        let existing = self.inner.borrow().hwnd;
        if !existing.is_invalid() {
            return existing;
        }
        let cls = to_wide_null(FLOAT_WIDGET_CLASS);
        let title = to_wide_null(TaskbarEmbed::NET_SPEED_FLOAT_TITLE);
        let raw_ptr = Rc::into_raw(Rc::clone(&self.inner));
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                PCWSTR(cls.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_POPUP,
                64,
                64,
                168,
                56,
                None,
                None,
                Some(hinstance()),
                Some(raw_ptr.cast()),
            )
            .unwrap_or_default()
        };
        if hwnd.is_invalid() {
            unsafe {
                drop(Rc::from_raw(raw_ptr));
            }
            return hwnd;
        }
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
        }
        self.inner.borrow_mut().hwnd = hwnd;
        TaskbarEmbed::prepare_per_pixel_layered(hwnd);
        hwnd
    }

    pub fn show_floating(&self, left_dip: f64, top_dip: f64) {
        let hwnd = self.ensure_window();
        if hwnd.is_invalid() {
            return;
        }
        let dpi = TaskbarEmbed::dpi_for(hwnd) as f64;
        let scale = dpi / 96.0;
        let px = (left_dip.clamp(-200.0, 4000.0) * scale).round() as i32;
        let py = (top_dip.clamp(-200.0, 3000.0) * scale).round() as i32;
        let pw = (Self::FLOAT_WIDTH_DIP * scale).round() as i32;
        let ph = (Self::FLOAT_HEIGHT_DIP * scale).round() as i32;
        {
            let mut st = self.inner.borrow_mut();
            st.left_dip = left_dip;
            st.top_dip = top_dip;
            st.visible = true;
        }
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                Some(windows::Win32::UI::WindowsAndMessaging::HWND_TOPMOST),
                px,
                py,
                pw,
                ph,
                SWP_NOACTIVATE,
            );
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        self.push_frame();
    }

    pub fn hide(&self) {
        let hwnd = {
            let mut st = self.inner.borrow_mut();
            st.visible = false;
            st.hwnd
        };
        if !hwnd.is_invalid() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }

    pub fn apply_appearance(&self, settings: &NetSpeedSettings) {
        let should_push = {
            let mut st = self.inner.borrow_mut();
            st.appearance = settings.clone();
            st.appearance.normalize();
            st.left_dip = settings.left;
            st.top_dip = settings.top;
            st.visible && !st.hwnd.is_invalid()
        };
        if should_push {
            self.push_frame();
        }
    }

    pub fn apply_theme(&self, theme: ThemeKind) {
        let should_push = {
            let mut st = self.inner.borrow_mut();
            st.theme = theme;
            st.visible && !st.hwnd.is_invalid()
        };
        if should_push {
            self.push_frame();
        }
    }

    pub fn apply_sample(&self, sample: &RateSample) {
        let should_push = {
            let mut st = self.inner.borrow_mut();
            st.adapter_missing = sample.adapter_missing;
            if sample.adapter_missing {
                st.down_bps = 0.0;
                st.up_bps = 0.0;
            } else {
                st.down_bps = sample.receive_bytes_per_second;
                st.up_bps = sample.send_bytes_per_second;
            }
            st.visible && !st.hwnd.is_invalid()
        };
        if should_push {
            self.push_frame();
        }
    }

    fn push_frame(&self) {
        let (hwnd, d2d, theme, app, down, up, miss) = {
            let st = self.inner.borrow();
            if st.hwnd.is_invalid() || !st.visible {
                return;
            }
            (
                st.hwnd,
                Rc::clone(&st.d2d),
                st.theme,
                st.appearance.clone(),
                st.down_bps,
                st.up_bps,
                st.adapter_missing,
            )
        };
        let (w, h) = match TaskbarEmbed::try_get_window_rect(hwnd) {
            Some(rc) => (rc.width().max(1), rc.height().max(1)),
            None => (168, 56),
        };
        let dpi = TaskbarEmbed::dpi_for(hwnd);
        let dark = theme == ThemeKind::Dark;
        let bg = if dark {
            D2D1_COLOR_F {
                r: 0.16,
                g: 0.16,
                b: 0.17,
                a: 0.92,
            }
        } else {
            D2D1_COLOR_F {
                r: 0.96,
                g: 0.96,
                b: 0.97,
                a: 0.92,
            }
        };
        let fg = if dark {
            D2D1_COLOR_F {
                r: 0.94,
                g: 0.94,
                b: 0.94,
                a: 1.0,
            }
        } else {
            D2D1_COLOR_F {
                r: 0.10,
                g: 0.10,
                b: 0.10,
                a: 1.0,
            }
        };
        let text = if miss {
            "↓ —\n↑ 选网卡".to_string()
        } else if !app.two_line {
            format!(
                "↓ {}  ↑ {}",
                RateFormatter::format_bytes_per_second(down),
                RateFormatter::format_bytes_per_second(up)
            )
        } else {
            format!(
                "↓ {}\n↑ {}",
                RateFormatter::format_bytes_per_second(down),
                RateFormatter::format_bytes_per_second(up)
            )
        };
        let scale = (dpi.max(96) as f32) / 96.0;
        let mut r = d2d.borrow_mut();
        if let Some(bytes) = r.draw_text_raw_into_dc(
            (w, h),
            bg,
            fg,
            &text,
            13.0 * scale,
            (10.0 * scale, 8.0 * scale),
        ) {
            let _ = TaskbarEmbed::try_push_per_pixel_layered(hwnd, w, h, bytes);
        }
    }
}

impl Drop for NetSpeedFloatWindow {
    fn drop(&mut self) {
        self.hide();
        let hwnd = {
            let mut st = self.inner.borrow_mut();
            let h = st.hwnd;
            st.hwnd = HWND::default();
            h
        };
        if !hwnd.is_invalid() {
            TaskbarEmbed::destroy_hwnd(hwnd);
        }
    }
}

unsafe extern "system" fn float_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
        if raw != 0 {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Rc::from_raw(raw as *const RefCell<FloatWidgetInner>));
            }
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let inner = unsafe { &*(ptr as *const RefCell<FloatWidgetInner>) };

    match msg {
        WM_LBUTTONDOWN => {
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), lparam);
            }
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            if let Some(rc) = TaskbarEmbed::try_get_window_rect(hwnd) {
                let dpi = TaskbarEmbed::dpi_for(hwnd) as f64;
                let scale = dpi / 96.0;
                let left = (rc.left as f64) / scale;
                let top = (rc.top as f64) / scale;
                let controller = {
                    let mut st = inner.borrow_mut();
                    st.left_dip = left;
                    st.top_dip = top;
                    st.controller
                };
                post_boxed(controller, WM_APP_NETSPEED_POS, 0, NetSpeedPosMsg { left, top });
            }
            LRESULT(0)
        }
        WM_RBUTTONUP | WM_CONTEXTMENU => {
            let controller = inner.borrow().controller;
            show_tray_chip_menu(hwnd, controller, false);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
