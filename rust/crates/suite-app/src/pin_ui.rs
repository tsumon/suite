use std::cell::RefCell;
use std::rc::Rc;

use suite_capture::{ImageClipboard, MonitorEnumerator, PixelBuffer, save_png_atomic};
use suite_contracts::ThemeKind;
use suite_platform::TaskbarEmbed;
use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_DELETE, VK_ESCAPE, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    GetCursorPos, GetWindowLongPtrW, GWLP_USERDATA, HMENU, HTCAPTION, HWND_TOPMOST, KillTimer,
    MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, PostMessageW, SW_SHOW, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_COMMAND,
    WM_CONTEXTMENU, WM_DESTROY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCDESTROY, WM_NCLBUTTONDOWN, WM_RBUTTONUP, WM_TIMER,
    WS_CHILD, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::PCWSTR;

const WM_MOUSELEAVE: u32 = 0x02A3;

use crate::d2d::D2dRenderer;
use crate::messages::{AppCommand, WM_APP_PIN_CHANGED, WM_APP_PIN_MANAGE_CLOSED, post_command};
use crate::win32_util::{
    confirm_warning, create_ui_font, hinstance, register_window_class, save_png_dialog,
    set_control_font,
};

const PIN_WND_CLASS: &str = "SuitePinWindowWnd";
const PIN_MANAGE_CLASS: &str = "SuitePinManageWnd";
const GLOW_PAD: i32 = 22;
const TIMER_HUE: usize = 1;

#[derive(Clone)]
pub struct PinInfo {
    pub index: usize,
    pub is_click_through: bool,
}

struct PinInner {
    hwnd: HWND,
    controller: HWND,
    d2d: Rc<RefCell<D2dRenderer>>,
    image: PixelBuffer,
    base_w_dip: f64,
    base_h_dip: f64,
    scale: f64,
    opacity: f32,
    phase: f32,
    hovered: bool,
    tracking_leave: bool,
    click_through: bool,
    theme: ThemeKind,
}

pub struct PinWindow {
    inner: Rc<RefCell<PinInner>>,
}

impl PinWindow {
    pub fn create(
        id: u64,
        controller: HWND,
        d2d: Rc<RefCell<D2dRenderer>>,
        image: PixelBuffer,
        pos_dip: Option<(f64, f64)>,
        dpi: (u32, u32),
        theme: ThemeKind,
    ) -> Option<Rc<Self>> {
        register_window_class(PIN_WND_CLASS, Some(pin_wnd_proc), false, None);
        let dx = dpi.0.max(96) as f64;
        let dy = dpi.1.max(96) as f64;
        let base_w_dip = (image.width as f64 * 96.0 / dx).max(1.0);
        let base_h_dip = (image.height as f64 * 96.0 / dy).max(1.0);
        let (cx, cy) = MonitorEnumerator::cursor_pos();
        let (px, py) = match pos_dip {
            Some((l, t)) => (
                (l * dx / 96.0).round() as i32 - GLOW_PAD,
                (t * dy / 96.0).round() as i32 - GLOW_PAD,
            ),
            _ => (cx - (image.width / 2) - GLOW_PAD, cy - (image.height / 2) - GLOW_PAD),
        };

        let inner = Rc::new(RefCell::new(PinInner {
            hwnd: HWND::default(),
            controller,
            d2d,
            image,
            base_w_dip,
            base_h_dip,
            scale: 1.0,
            opacity: 1.0,
            phase: 0.0,
            hovered: false,
            tracking_leave: false,
            click_through: false,
            theme,
        }));

        let cls = to_wide_null(PIN_WND_CLASS);
        let title = to_wide_null(&format!("Suite Pin {id}"));
        let raw_ptr = Rc::into_raw(Rc::clone(&inner));
        let total_w = (base_w_dip * dx / 96.0).round() as i32 + GLOW_PAD * 2;
        let total_h = (base_h_dip * dy / 96.0).round() as i32 + GLOW_PAD * 2;
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                PCWSTR(cls.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_POPUP,
                px,
                py,
                total_w,
                total_h,
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
            return None;
        }
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
        }
        inner.borrow_mut().hwnd = hwnd;
        TaskbarEmbed::prepare_per_pixel_layered(hwnd);
        render_pin_inner(&inner);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetTimer(Some(hwnd), TIMER_HUE, 50, None);
        }
        Some(Rc::new(Self { inner }))
    }

    pub fn is_alive(&self) -> bool {
        let hwnd = self.inner.borrow().hwnd;
        !hwnd.is_invalid() && unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(hwnd)).as_bool() }
    }

    pub fn is_click_through(&self) -> bool {
        self.inner.borrow().click_through
    }

    pub fn apply_theme(&self, theme: ThemeKind) {
        self.inner.borrow_mut().theme = theme;
        render_pin_inner(&self.inner);
    }

    pub fn set_click_through(&self, enabled: bool) {
        let (hwnd, controller) = {
            let mut st = self.inner.borrow_mut();
            st.click_through = enabled;
            (st.hwnd, st.controller)
        };
        TaskbarEmbed::set_click_through(hwnd, enabled);
        unsafe {
            let _ = PostMessageW(Some(controller), WM_APP_PIN_CHANGED, WPARAM(0), LPARAM(0));
        }
    }

    pub fn focus(&self) {
        let hwnd = self.inner.borrow().hwnd;
        if !hwnd.is_invalid() {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE,
                );
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }

    pub fn close(&self) {
        let hwnd = {
            let mut st = self.inner.borrow_mut();
            let h = st.hwnd;
            st.hwnd = HWND::default();
            st.image.release_pixels();
            h
        };
        if !hwnd.is_invalid() {
            unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_HUE);
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

fn render_pin_inner(inner: &Rc<RefCell<PinInner>>) {
    let (hwnd, d2d, base_w, base_h, scale, opacity, phase, hovered, theme, img_w_orig, img_h_orig) = {
        let st = inner.borrow();
        if st.hwnd.is_invalid() || st.image.bgra.is_empty() {
            return;
        }
        (
            st.hwnd,
            Rc::clone(&st.d2d),
            st.base_w_dip,
            st.base_h_dip,
            st.scale,
            st.opacity,
            st.phase,
            st.hovered,
            st.theme,
            st.image.width,
            st.image.height,
        )
    };
    let _ = (img_w_orig, img_h_orig);
    let dpi = TaskbarEmbed::dpi_for(hwnd) as f64;
    let img_w = (base_w * scale * dpi / 96.0).round().max(4.0) as i32;
    let img_h = (base_h * scale * dpi / 96.0).round().max(4.0) as i32;

    let far_alpha = if theme == ThemeKind::Dark { 0.32 } else { 0.48 }
        + if hovered { 0.08 } else { 0.0 };
    let near_alpha = if theme == ThemeKind::Dark { 0.42 } else { 0.58 }
        + if hovered { 0.08 } else { 0.0 };

    let st_ref = inner.borrow();
    let frame = {
        let mut r = d2d.borrow_mut();
        r.render_pin_frame(
            &st_ref.image,
            img_w,
            img_h,
            GLOW_PAD,
            phase,
            far_alpha,
            near_alpha,
            opacity,
        )
    };
    drop(st_ref);

    if let Some((tw, th, bytes)) = frame {
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                tw,
                th,
                SWP_NOMOVE | SWP_NOACTIVATE,
            );
        }
        let _ = TaskbarEmbed::try_push_per_pixel_layered(hwnd, tw, th, &bytes);
    }
}

unsafe extern "system" fn pin_wnd_proc(
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
                let rc = Rc::from_raw(raw as *const RefCell<PinInner>);
                rc.borrow_mut().image.release_pixels();
                drop(rc);
            }
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let inner_rc = unsafe {
        let r = Rc::from_raw(ptr as *const RefCell<PinInner>);
        let cloned = Rc::clone(&r);
        std::mem::forget(r);
        cloned
    };

    match msg {
        WM_TIMER if wparam.0 == TIMER_HUE => {
            {
                let mut st = inner_rc.borrow_mut();
                st.phase = (st.phase + 0.008) % 1.0;
            }
            render_pin_inner(&inner_rc);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let need_track = {
                let mut st = inner_rc.borrow_mut();
                let changed = !st.hovered;
                st.hovered = true;
                let tr = !st.tracking_leave;
                st.tracking_leave = true;
                if changed || tr { (tr, changed) } else { (false, false) }
            };
            if need_track.0 {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe {
                    let _ = TrackMouseEvent(&mut tme);
                }
            }
            if need_track.1 {
                render_pin_inner(&inner_rc);
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            {
                let mut st = inner_rc.borrow_mut();
                st.hovered = false;
                st.tracking_leave = false;
            }
            render_pin_inner(&inner_rc);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let ct = inner_rc.borrow().click_through;
            if !ct {
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), lparam);
                }
            }
            LRESULT(0)
        }
        WM_MBUTTONDOWN => {
            let (next, controller) = {
                let mut st = inner_rc.borrow_mut();
                st.click_through = !st.click_through;
                (st.click_through, st.controller)
            };
            TaskbarEmbed::set_click_through(hwnd, next);
            unsafe {
                let _ = PostMessageW(Some(controller), WM_APP_PIN_CHANGED, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as u16) as i16;
            let alt_down = unsafe { GetKeyState(i32::from(VK_MENU.0)) } < 0;
            if alt_down {
                {
                    let mut st = inner_rc.borrow_mut();
                    let step = if delta > 0 { 0.08f32 } else { -0.08f32 };
                    st.opacity = (st.opacity + step).clamp(0.15, 1.0);
                }
                render_pin_inner(&inner_rc);
            } else {
                let (old_scale, new_scale) = {
                    let mut st = inner_rc.borrow_mut();
                    let old = st.scale;
                    let factor = if delta > 0 { 1.1 } else { 1.0 / 1.1 };
                    let next = (old * factor).clamp(0.15, 8.0);
                    st.scale = next;
                    (old, next)
                };
                if let Some(rc) = TaskbarEmbed::try_get_window_rect(hwnd) {
                    let mut pt = POINT::default();
                    unsafe {
                        let _ = GetCursorPos(&mut pt);
                    }
                    let rel_x = (pt.x - rc.left - GLOW_PAD) as f64;
                    let rel_y = (pt.y - rc.top - GLOW_PAD) as f64;
                    let ratio = 1.0 - (new_scale / old_scale);
                    let new_left = rc.left + (rel_x * ratio).round() as i32;
                    let new_top = rc.top + (rel_y * ratio).round() as i32;
                    unsafe {
                        let _ = SetWindowPos(
                            hwnd,
                            Some(HWND_TOPMOST),
                            new_left,
                            new_top,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
                render_pin_inner(&inner_rc);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let vk = wparam.0 as u16;
            let ct = inner_rc.borrow().click_through;
            if !ct && (vk == VK_ESCAPE.0 || vk == VK_DELETE.0) {
                let controller = inner_rc.borrow().controller;
                unsafe {
                    let _ = DestroyWindow(hwnd);
                    let _ = PostMessageW(Some(controller), WM_APP_PIN_CHANGED, WPARAM(0), LPARAM(0));
                }
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_RBUTTONUP | WM_CONTEXTMENU => {
            show_pin_context_menu(hwnd, &inner_rc);
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_HUE);
            }
            let controller = inner_rc.borrow().controller;
            unsafe {
                let _ = PostMessageW(Some(controller), WM_APP_PIN_CHANGED, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn show_pin_context_menu(hwnd: HWND, inner_rc: &Rc<RefCell<PinInner>>) {
    let ct = inner_rc.borrow().click_through;
    let cmd = unsafe {
        let Ok(hmenu) = CreatePopupMenu() else {
            return;
        };
        let add = |id: usize, text: &str, flags: u32| {
            let w = to_wide_null(text);
            let _ = AppendMenuW(
                hmenu,
                windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS(MF_STRING.0 | flags),
                id,
                PCWSTR(w.as_ptr()),
            );
        };
        add(1, "不透明度 100%", 0);
        add(2, "不透明度 75%", 0);
        add(3, "不透明度 50%", 0);
        add(4, "不透明度 25%", 0);
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        add(
            5,
            "鼠标穿透 (Ctrl+Shift+Q)",
            if ct { MF_CHECKED.0 } else { MF_UNCHECKED.0 },
        );
        add(6, "复制图片", 0);
        add(7, "保存图片...", 0);
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        add(8, "关闭", 0);

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let c = TrackPopupMenu(
            hmenu,
            TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x,
            pt.y,
            Some(0),
            hwnd,
            None,
        );
        let _ = DestroyMenu(hmenu);
        c.0
    };

    match cmd {
        1 => {
            inner_rc.borrow_mut().opacity = 1.0;
            render_pin_inner(inner_rc);
        }
        2 => {
            inner_rc.borrow_mut().opacity = 0.75;
            render_pin_inner(inner_rc);
        }
        3 => {
            inner_rc.borrow_mut().opacity = 0.50;
            render_pin_inner(inner_rc);
        }
        4 => {
            inner_rc.borrow_mut().opacity = 0.25;
            render_pin_inner(inner_rc);
        }
        5 => {
            let (next, controller) = {
                let mut st = inner_rc.borrow_mut();
                st.click_through = !st.click_through;
                (st.click_through, st.controller)
            };
            TaskbarEmbed::set_click_through(hwnd, next);
            unsafe {
                let _ = PostMessageW(Some(controller), WM_APP_PIN_CHANGED, WPARAM(0), LPARAM(0));
            }
        }
        6 => {
            let st = inner_rc.borrow();
            let _ = ImageClipboard::copy(&st.image);
        }
        7 => {
            let img_copy = inner_rc.borrow().image.clone();
            if let Some(path) = save_png_dialog(hwnd, "", "Suite-Pin.png") {
                let _ = suite_capture::save_png_to_path(&img_copy, &path);
            } else {
                let _ = save_png_atomic(&img_copy, None);
            }
        }
        8 => {
            let controller = inner_rc.borrow().controller;
            unsafe {
                let _ = DestroyWindow(hwnd);
                let _ = PostMessageW(Some(controller), WM_APP_PIN_CHANGED, WPARAM(0), LPARAM(0));
            }
        }
        _ => {}
    }
}

pub struct PinboardService {
    controller: HWND,
    d2d: Rc<RefCell<D2dRenderer>>,
    pins: Vec<Rc<PinWindow>>,
    next_id: u64,
    theme: ThemeKind,
}

impl PinboardService {
    pub fn new(controller: HWND, d2d: Rc<RefCell<D2dRenderer>>) -> Self {
        Self {
            controller,
            d2d,
            pins: Vec::new(),
            next_id: 1,
            theme: ThemeKind::Light,
        }
    }

    pub fn prune_closed(&mut self) {
        self.pins.retain(|p| p.is_alive());
    }

    pub fn pin(
        &mut self,
        image: PixelBuffer,
        left_dip: Option<f64>,
        top_dip: Option<f64>,
        dpi_x: u32,
        dpi_y: u32,
    ) {
        self.prune_closed();
        let id = self.next_id;
        self.next_id += 1;
        if let Some(win) = PinWindow::create(
            id,
            self.controller,
            Rc::clone(&self.d2d),
            image,
            left_dip.zip(top_dip),
            (dpi_x, dpi_y),
            self.theme,
        ) {
            self.pins.push(win);
        }
    }

    pub fn try_pin_from_clipboard(&mut self) -> Result<(), String> {
        let Some(img) = ImageClipboard::try_get() else {
            return Err("剪贴板里没有图片。".to_string());
        };
        let dpi = MonitorEnumerator::dpi_near_cursor();
        self.pin(img, None, None, dpi, dpi);
        Ok(())
    }

    pub fn apply_theme(&mut self, theme: ThemeKind) {
        self.theme = theme;
        self.prune_closed();
        for p in &self.pins {
            p.apply_theme(theme);
        }
    }

    pub fn clear_click_through(&mut self) {
        self.prune_closed();
        for p in &self.pins {
            if p.is_click_through() {
                p.set_click_through(false);
            }
        }
    }

    pub fn try_toggle_top_click_through(&mut self) -> Result<(), String> {
        self.prune_closed();
        let Some(top) = self.pins.last() else {
            return Err("当前没有贴图。".to_string());
        };
        top.set_click_through(!top.is_click_through());
        Ok(())
    }

    pub fn list(&mut self) -> Vec<PinInfo> {
        self.prune_closed();
        self.pins
            .iter()
            .enumerate()
            .map(|(i, p)| PinInfo {
                index: i + 1,
                is_click_through: p.is_click_through(),
            })
            .collect()
    }

    pub fn focus_by_index(&mut self, one_based: usize) {
        self.prune_closed();
        if let Some(p) = one_based.checked_sub(1).and_then(|i| self.pins.get(i)) {
            p.focus();
        }
    }

    pub fn toggle_through_by_index(&mut self, one_based: usize) {
        self.prune_closed();
        if let Some(p) = one_based.checked_sub(1).and_then(|i| self.pins.get(i)) {
            p.set_click_through(!p.is_click_through());
        }
    }

    pub fn close_by_index(&mut self, one_based: usize) {
        self.prune_closed();
        if let Some(idx) = one_based.checked_sub(1)
            && idx < self.pins.len()
        {
            let p = self.pins.remove(idx);
            p.close();
        }
    }

    pub fn close_all(&mut self) {
        for p in self.pins.drain(..) {
            p.close();
        }
    }
}

struct PinManageState {
    hwnd: HWND,
    controller: HWND,
    font: windows::Win32::Graphics::Gdi::HFONT,
    children: Vec<HWND>,
    empty_label: HWND,
}

pub struct PinManageWindow {
    state: Rc<RefCell<PinManageState>>,
}

impl PinManageWindow {
    pub fn show_or_focus(existing: Option<&Self>, controller: HWND, pins: &[PinInfo]) -> Rc<Self> {
        if let Some(win) = existing {
            let hwnd = win.state.borrow().hwnd;
            if !hwnd.is_invalid() {
                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
                win.reload(pins);
                return Rc::new(Self {
                    state: Rc::clone(&win.state),
                });
            }
        }

        register_window_class(PIN_MANAGE_CLASS, Some(pin_manage_proc), false, None);
        let font = create_ui_font(14, false, false, "Microsoft YaHei UI");
        let state = Rc::new(RefCell::new(PinManageState {
            hwnd: HWND::default(),
            controller,
            font,
            children: Vec::new(),
            empty_label: HWND::default(),
        }));
        let cls = to_wide_null(PIN_MANAGE_CLASS);
        let title = to_wide_null("贴图管理");
        let raw_ptr = Rc::into_raw(Rc::clone(&state));
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(cls.as_ptr()),
                PCWSTR(title.as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::WS_OVERLAPPED
                    | windows::Win32::UI::WindowsAndMessaging::WS_CAPTION
                    | windows::Win32::UI::WindowsAndMessaging::WS_SYSMENU,
                200,
                200,
                420,
                380,
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
        } else {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
            }
            state.borrow_mut().hwnd = hwnd;
            build_pin_manage_static(hwnd, font);
            let win = Rc::new(Self { state });
            win.reload(pins);
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            return win;
        }
        Rc::new(Self { state })
    }

    pub fn reload(&self, pins: &[PinInfo]) {
        let (hwnd, font, old_children, empty_lbl) = {
            let mut st = self.state.borrow_mut();
            let c = std::mem::take(&mut st.children);
            (st.hwnd, st.font, c, st.empty_label)
        };
        if hwnd.is_invalid() {
            return;
        }
        for h in old_children {
            unsafe {
                let _ = DestroyWindow(h);
            }
        }
        let mut created = Vec::new();
        if !empty_lbl.is_invalid() {
            unsafe {
                let _ = ShowWindow(
                    empty_lbl,
                    if pins.is_empty() {
                        SW_SHOW
                    } else {
                        windows::Win32::UI::WindowsAndMessaging::SW_HIDE
                    },
                );
            }
        }
        let btn_cls = to_wide_null("BUTTON");
        let lbl_cls = to_wide_null("STATIC");
        for (idx, p) in pins.iter().take(7).enumerate() {
            let y = 16 + (idx as i32) * 38;
            let txt = to_wide_null(&format!("贴图 {}", p.index));
            unsafe {
                if let Ok(lbl) = CreateWindowExW(
                    Default::default(),
                    PCWSTR(lbl_cls.as_ptr()),
                    PCWSTR(txt.as_ptr()),
                    WS_CHILD | WS_VISIBLE,
                    16,
                    y + 4,
                    90,
                    24,
                    Some(hwnd),
                    None,
                    Some(hinstance()),
                    None,
                ) {
                    set_control_font(lbl, font);
                    created.push(lbl);
                }

                let mk_btn = |label: &str, x: i32, w: i32, cmd_id: usize| -> Option<HWND> {
                    let wt = to_wide_null(label);
                    CreateWindowExW(
                        Default::default(),
                        PCWSTR(btn_cls.as_ptr()),
                        PCWSTR(wt.as_ptr()),
                        WS_CHILD | WS_VISIBLE,
                        x,
                        y,
                        w,
                        28,
                        Some(hwnd),
                        Some(HMENU(cmd_id as *mut _)),
                        Some(hinstance()),
                        None,
                    )
                    .ok()
                };
                if let Some(b1) = mk_btn("定位", 120, 68, 1000 + p.index) {
                    set_control_font(b1, font);
                    created.push(b1);
                }
                let t_label = if p.is_click_through { "穿透关" } else { "穿透开" };
                if let Some(b2) = mk_btn(t_label, 196, 86, 2000 + p.index) {
                    set_control_font(b2, font);
                    created.push(b2);
                }
                if let Some(b3) = mk_btn("关闭", 290, 68, 3000 + p.index) {
                    set_control_font(b3, font);
                    created.push(b3);
                }
            }
        }
        self.state.borrow_mut().children = created;
    }

    pub fn close(&self) {
        let hwnd = self.state.borrow().hwnd;
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}

fn build_pin_manage_static(hwnd: HWND, font: windows::Win32::Graphics::Gdi::HFONT) {
    let btn_cls = to_wide_null("BUTTON");
    let lbl_cls = to_wide_null("STATIC");
    unsafe {
        if let Ok(empty) = CreateWindowExW(
            Default::default(),
            PCWSTR(lbl_cls.as_ptr()),
            PCWSTR(to_wide_null("当前没有贴图。").as_ptr()),
            WS_CHILD | WS_VISIBLE,
            16,
            24,
            240,
            24,
            Some(hwnd),
            None,
            Some(hinstance()),
            None,
        ) {
            set_control_font(empty, font);
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                let st = &*(ptr as *const RefCell<PinManageState>);
                st.borrow_mut().empty_label = empty;
            }
        }
        if let Ok(b_all) = CreateWindowExW(
            Default::default(),
            PCWSTR(btn_cls.as_ptr()),
            PCWSTR(to_wide_null("全部关闭").as_ptr()),
            WS_CHILD | WS_VISIBLE,
            16,
            298,
            100,
            30,
            Some(hwnd),
            Some(HMENU(9001 as *mut _)),
            Some(hinstance()),
            None,
        ) {
            set_control_font(b_all, font);
        }
        if let Ok(b_close) = CreateWindowExW(
            Default::default(),
            PCWSTR(btn_cls.as_ptr()),
            PCWSTR(to_wide_null("关闭窗口").as_ptr()),
            WS_CHILD | WS_VISIBLE,
            292,
            298,
            100,
            30,
            Some(hwnd),
            Some(HMENU(9002 as *mut _)),
            Some(hinstance()),
            None,
        ) {
            set_control_font(b_close, font);
        }
    }
}

unsafe extern "system" fn pin_manage_proc(
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
                let rc = Rc::from_raw(raw as *const RefCell<PinManageState>);
                let font = rc.borrow().font;
                if !font.is_invalid() {
                    let _ = windows::Win32::Graphics::Gdi::DeleteObject(font.into());
                }
                drop(rc);
            }
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let st_ref = unsafe { &*(ptr as *const RefCell<PinManageState>) };

    match msg {
        WM_COMMAND => {
            let id = wparam.0 & 0xFFFF;
            let controller = st_ref.borrow().controller;
            if (1001..2000).contains(&id) {
                post_command(controller, AppCommand::FocusPin, (id - 1000) as isize);
            } else if (2001..3000).contains(&id) {
                post_command(controller, AppCommand::TogglePinThrough, (id - 2000) as isize);
            } else if (3001..4000).contains(&id) {
                post_command(controller, AppCommand::ClosePin, (id - 3000) as isize);
            } else if id == 9001 {
                if confirm_warning(hwnd, "贴图管理", "关闭全部贴图？") {
                    post_command(controller, AppCommand::CloseAllPins, 0);
                }
            } else if id == 9002 {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let controller = st_ref.borrow().controller;
            unsafe {
                let _ = PostMessageW(Some(controller), WM_APP_PIN_MANAGE_CLOSED, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
