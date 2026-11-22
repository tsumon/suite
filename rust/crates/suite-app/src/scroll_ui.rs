use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use suite_capture::{
    CaptureUx, MonitorCapture, MonitorEnumerator, PixelRect, ScreenCapturePipeline, ScrollCapture,
    ScrollCaptureOptions, WindowPicker,
};
use suite_platform::TaskbarEmbed;
use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, EndPaint, FillRect, GetStockObject, HBRUSH, HOLLOW_BRUSH, LineTo,
    MoveToEx, PAINTSTRUCT, PS_SOLID, Rectangle, SRCCOPY, SelectObject, SetBkColor, SetBkMode,
    SetTextColor, TRANSPARENT, TextOutW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, VK_ESCAPE, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, GWLP_USERDATA,
    HWND_TOPMOST, IDC_CROSS, LoadCursorW, SW_SHOW, SW_SHOWNOACTIVATE, SetForegroundWindow,
    SetWindowLongPtrW, SetWindowTextW, ShowWindow, WM_CTLCOLORBTN, WM_CTLCOLORSTATIC,
    WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCDESTROY,
    WM_PAINT, WM_RBUTTONUP, WS_CHILD, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};
use windows::core::PCWSTR;

use crate::color_pick_ui::blit_pixel_buffer;
use crate::d2d::D2dRenderer;
use crate::messages::{
    ScrollPickMsg, WM_APP_SCROLL_DONE, WM_APP_SCROLL_PICKED, WM_APP_SCROLL_PROGRESS, post_boxed,
};
use crate::win32_util::{
    apply_window_theme, create_ui_font, dpi_for_cursor, hinstance, invalidate_all,
    register_window_class, s_px, set_control_font,
};

const SCROLL_PICK_CLASS: &str = "SuiteScrollPickOverlayWnd";
const SCROLL_BORDER_CLASS: &str = "SuiteScrollBorderWnd";
const SCROLL_PILL_CLASS: &str = "SuiteScrollStatusPillWnd";

struct SharedScrollPick {
    controller: HWND,
    windows: Vec<HWND>,
    monitors: Vec<suite_capture::MonitorInfo>,
    hover: Option<PixelRect>,
    press: Option<(i32, i32)>,
    drag: Option<PixelRect>,
    dragging: bool,
    finished: bool,
}

struct PickWndState {
    shared: Rc<RefCell<SharedScrollPick>>,
    capture: MonitorCapture,
    font: windows::Win32::Graphics::Gdi::HFONT,
}

pub struct ScrollCaptureSession {
    picker_shared: Option<Rc<RefCell<SharedScrollPick>>>,
    border_hwnd: HWND,
    pill_hwnd: HWND,
    pill_label: HWND,
    cancel_flag: Arc<AtomicBool>,
    finish_flag: Arc<AtomicBool>,
    pill_font: windows::Win32::Graphics::Gdi::HFONT,
}

impl ScrollCaptureSession {
    pub fn begin_pick(controller: HWND) -> Result<Self, String> {
        let frames = ScreenCapturePipeline::capture_all()?;
        let cross = unsafe { LoadCursorW(None, IDC_CROSS).ok() };
        register_window_class(SCROLL_PICK_CLASS, Some(scroll_pick_proc), false, cross);

        let monitors: Vec<_> = frames.iter().map(|f| f.monitor.clone()).collect();
        let shared = Rc::new(RefCell::new(SharedScrollPick {
            controller,
            windows: Vec::with_capacity(frames.len()),
            monitors,
            hover: None,
            press: None,
            drag: None,
            dragging: false,
            finished: false,
        }));

        let cursor = MonitorEnumerator::cursor_pos();
        let cls = to_wide_null(SCROLL_PICK_CLASS);
        let title = to_wide_null("Suite ScrollPick");
        let mut first_hwnd = HWND::default();

        for cap in frames {
            let b = cap.monitor.bounds;
            let font = create_ui_font(13, false, false, "Microsoft YaHei UI");
            let state = Rc::new(RefCell::new(PickWndState {
                shared: Rc::clone(&shared),
                capture: cap,
                font,
            }));
            let raw_ptr = Rc::into_raw(state);
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                    PCWSTR(cls.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    WS_POPUP,
                    b.x,
                    b.y,
                    b.width,
                    b.height,
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
                continue;
            }
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            shared.borrow_mut().windows.push(hwnd);
            if first_hwnd.is_invalid() || b.contains(cursor.0, cursor.1) {
                first_hwnd = hwnd;
            }
        }

        if !first_hwnd.is_invalid() {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                    first_hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE
                        | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE,
                );
                let _ = SetForegroundWindow(first_hwnd);
                let _ = SetCapture(first_hwnd);
            }
        }

        Ok(Self {
            picker_shared: Some(shared),
            border_hwnd: HWND::default(),
            pill_hwnd: HWND::default(),
            pill_label: HWND::default(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            finish_flag: Arc::new(AtomicBool::new(false)),
            pill_font: Default::default(),
        })
    }

    pub fn start_scrolling(
        &mut self,
        controller: HWND,
        d2d: &Rc<RefCell<D2dRenderer>>,
        rect: PixelRect,
    ) {
        self.picker_shared = None;
        register_window_class(SCROLL_BORDER_CLASS, Some(border_proc), false, None);
        register_window_class(SCROLL_PILL_CLASS, Some(pill_proc), false, None);

        // Border window sits 3px OUTSIDE rect and is click-through so BitBlt inside rect captures target cleanly.
        let bw = rect.width + 6;
        let bh = rect.height + 6;
        let bcls = to_wide_null(SCROLL_BORDER_CLASS);
        let btitle = to_wide_null("Suite ScrollBorder");
        let border = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                PCWSTR(bcls.as_ptr()),
                PCWSTR(btitle.as_ptr()),
                WS_POPUP,
                rect.x - 3,
                rect.y - 3,
                bw,
                bh,
                None,
                None,
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        if !border.is_invalid() {
            TaskbarEmbed::prepare_per_pixel_layered(border);
            unsafe {
                let _ = ShowWindow(border, SW_SHOWNOACTIVATE);
            }
            let mut bgra = vec![0u8; (bw * bh * 4) as usize];
            for y in 0..bh {
                for x in 0..bw {
                    let is_edge = x < 3 || x >= bw - 3 || y < 3 || y >= bh - 3;
                    if is_edge {
                        let i = ((y * bw + x) * 4) as usize;
                        bgra[i] = 0xF0;     // B
                        bgra[i + 1] = 0x80; // G
                        bgra[i + 2] = 0x20; // R
                        bgra[i + 3] = 0xFF; // A
                    }
                }
            }
            let _ = TaskbarEmbed::try_push_per_pixel_layered(border, bw, bh, &bgra);
        }
        self.border_hwnd = border;

        // Floating dark pill below/above rect with live progress, "完成" & "取消" buttons.
        let dpi = dpi_for_cursor();
        let s = |d: i32| s_px(d, dpi);
        let pill_w = s(460);
        let pill_h = s(38);
        let pill_x = rect.x.max(8);
        let pill_y = (rect.bottom() + 8).min(980);
        let pcls = to_wide_null(SCROLL_PILL_CLASS);
        let ptitle = to_wide_null("Suite ScrollStatus");
        let bg_brush = unsafe { CreateSolidBrush(COLORREF(0x001F_1F1F)) };
        let pill_state = Box::into_raw(Box::new((
            Arc::clone(&self.finish_flag),
            Arc::clone(&self.cancel_flag),
            bg_brush,
        )));
        let pill = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                PCWSTR(pcls.as_ptr()),
                PCWSTR(ptitle.as_ptr()),
                WS_POPUP,
                pill_x,
                pill_y,
                pill_w,
                pill_h,
                None,
                None,
                Some(hinstance()),
                Some(pill_state.cast()),
            )
            .unwrap_or_default()
        };
        let font = create_ui_font(s(13), false, false, "Microsoft YaHei UI");
        self.pill_font = font;
        if pill.is_invalid() {
            unsafe {
                let _ = DeleteObject(bg_brush.into());
                drop(Box::from_raw(pill_state));
            }
        } else {
            apply_window_theme(pill, true);
            unsafe {
                SetWindowLongPtrW(pill, GWLP_USERDATA, pill_state as isize);
                let _ = ShowWindow(pill, SW_SHOWNOACTIVATE);
                let lbl = CreateWindowExW(
                    Default::default(),
                    PCWSTR(to_wide_null("STATIC").as_ptr()),
                    PCWSTR(to_wide_null(ScrollCapture::SCROLL_HINT).as_ptr()),
                    WS_CHILD | WS_VISIBLE,
                    s(12),
                    s(10),
                    s(316),
                    s(20),
                    Some(pill),
                    None,
                    Some(hinstance()),
                    None,
                )
                .unwrap_or_default();
                set_control_font(lbl, font);
                self.pill_label = lbl;
                let b_done = CreateWindowExW(
                    Default::default(),
                    PCWSTR(to_wide_null("BUTTON").as_ptr()),
                    PCWSTR(to_wide_null("完成").as_ptr()),
                    WS_CHILD | WS_VISIBLE,
                    s(334),
                    s(6),
                    s(56),
                    s(26),
                    Some(pill),
                    Some(windows::Win32::UI::WindowsAndMessaging::HMENU(std::ptr::dangling_mut())),
                    Some(hinstance()),
                    None,
                )
                .unwrap_or_default();
                set_control_font(b_done, font);
                let b_cancel = CreateWindowExW(
                    Default::default(),
                    PCWSTR(to_wide_null("BUTTON").as_ptr()),
                    PCWSTR(to_wide_null("取消").as_ptr()),
                    WS_CHILD | WS_VISIBLE,
                    s(396),
                    s(6),
                    s(56),
                    s(26),
                    Some(pill),
                    Some(windows::Win32::UI::WindowsAndMessaging::HMENU(2 as *mut _)),
                    Some(hinstance()),
                    None,
                )
                .unwrap_or_default();
                set_control_font(b_cancel, font);
            }
        }
        self.pill_hwnd = pill;
        let _ = d2d;

        let cancel = Arc::clone(&self.cancel_flag);
        let finish = Arc::clone(&self.finish_flag);
        let ctrl_raw = controller.0 as isize;
        let _ = std::thread::Builder::new()
            .name("suite-scroll-capture".to_string())
            .spawn(move || {
                let ctrl = HWND(ctrl_raw as *mut _);
                let res = ScrollCapture::capture_region(
                    rect,
                    &ScrollCaptureOptions::default(),
                    &cancel,
                    &finish,
                    |h| unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                            Some(ctrl),
                            WM_APP_SCROLL_PROGRESS,
                            WPARAM(h as usize),
                            LPARAM(0),
                        );
                    },
                );
                post_boxed(ctrl, WM_APP_SCROLL_DONE, 0, res);
            });
    }

    pub fn update_progress(&self, height_px: i32) {
        if !self.pill_label.is_invalid() {
            let txt = to_wide_null(&format!("已拼接 {height_px} px · 停滚 1.5 秒自动完成"));
            unsafe {
                let _ = SetWindowTextW(self.pill_label, PCWSTR(txt.as_ptr()));
            }
        }
    }

    pub fn cancel(&mut self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
        if let Some(shared) = self.picker_shared.take() {
            close_scroll_pick(&shared, None);
        }
        if !self.border_hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(self.border_hwnd);
            }
            self.border_hwnd = HWND::default();
        }
        if !self.pill_hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(self.pill_hwnd);
            }
            self.pill_hwnd = HWND::default();
            self.pill_label = HWND::default();
        }
        if !self.pill_font.is_invalid() {
            unsafe {
                let _ = DeleteObject(self.pill_font.into());
            }
            self.pill_font = Default::default();
        }
    }
}

impl Drop for ScrollCaptureSession {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn close_scroll_pick(shared: &Rc<RefCell<SharedScrollPick>>, rect: Option<PixelRect>) {
    let (controller, windows) = {
        let mut s = shared.borrow_mut();
        if s.finished {
            return;
        }
        s.finished = true;
        (s.controller, std::mem::take(&mut s.windows))
    };
    unsafe {
        let _ = ReleaseCapture();
    }
    for w in windows {
        if !w.is_invalid() {
            unsafe {
                let _ = DestroyWindow(w);
            }
        }
    }
    post_boxed(controller, WM_APP_SCROLL_PICKED, 0, ScrollPickMsg { rect });
}

unsafe extern "system" fn scroll_pick_proc(
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
                let rc = Rc::from_raw(raw as *const RefCell<PickWndState>);
                {
                    let mut st = rc.borrow_mut();
                    st.capture.buffer.release_pixels();
                    if !st.font.is_invalid() {
                        let _ = DeleteObject(st.font.into());
                    }
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
    let state_rc = unsafe {
        let r = Rc::from_raw(ptr as *const RefCell<PickWndState>);
        let c = Rc::clone(&r);
        std::mem::forget(r);
        c
    };

    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_scroll_pick(hwnd, &state_rc.borrow());
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let shared = Rc::clone(&state_rc.borrow().shared);
            let windows_to_invalidate = {
                let mut s = shared.borrow_mut();
                if let Some((sx, sy)) = s.press {
                    if !s.dragging && CaptureUx::is_drag(sx, sy, vx, vy) {
                        s.dragging = true;
                        s.hover = None;
                    }
                    if s.dragging {
                        s.drag = Some(PixelRect::from_corners(sx, sy, vx, vy));
                    }
                    Some(s.windows.clone())
                } else {
                    let ignore: Vec<isize> = s.windows.iter().map(|h| h.0 as isize).collect();
                    let next = WindowPicker::hit(vx, vy, &ignore, &s.monitors);
                    let next_opt = (!next.is_empty()).then_some(next);
                    if next_opt != s.hover {
                        s.hover = next_opt;
                        Some(s.windows.clone())
                    } else {
                        None
                    }
                }
            };
            if let Some(ws) = windows_to_invalidate {
                for w in ws {
                    invalidate_all(w);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let shared = Rc::clone(&state_rc.borrow().shared);
            {
                let mut s = shared.borrow_mut();
                s.press = Some((vx, vy));
                s.dragging = false;
                s.drag = None;
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let shared = Rc::clone(&state_rc.borrow().shared);
            let chosen = {
                let mut s = shared.borrow_mut();
                let mut drag = s.drag;
                if s.dragging && let Some((sx, sy)) = s.press {
                    drag = Some(PixelRect::from_corners(sx, sy, vx, vy));
                }
                let hover = s.hover;
                s.press = None;
                let act = CaptureUx::decide_release(
                    s.dragging,
                    drag.map(|d| d.width).unwrap_or(0),
                    drag.map(|d| d.height).unwrap_or(0),
                    hover.is_some(),
                );
                match act {
                    suite_capture::ReleaseAction::CommitDrag => drag,
                    suite_capture::ReleaseAction::CommitHover => hover,
                    suite_capture::ReleaseAction::Stay => None,
                }
            };
            if let Some(rect) = chosen {
                close_scroll_pick(&shared, Some(rect));
            }
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            let shared = Rc::clone(&state_rc.borrow().shared);
            close_scroll_pick(&shared, None);
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 as u16 == VK_ESCAPE.0 => {
            let shared = Rc::clone(&state_rc.borrow().shared);
            close_scroll_pick(&shared, None);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn paint_scroll_pick(hwnd: HWND, st: &PickWndState) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    if hdc.is_invalid() {
        return;
    }
    let b = st.capture.monitor.bounds;
    let cw = b.width.max(1);
    let ch = b.height.max(1);
    let highlight = {
        let s = st.shared.borrow();
        if s.dragging { s.drag } else { s.hover }
    };

    unsafe {
        let mem_dc = CreateCompatibleDC(Some(hdc));
        let mem_bmp = CreateCompatibleBitmap(hdc, cw, ch);
        let old_bmp = SelectObject(mem_dc, mem_bmp.into());
        blit_pixel_buffer(mem_dc, &st.capture.buffer, cw, ch);

        if let Some(hl) = highlight {
            let local = hl.intersect(b);
            if !local.is_empty() {
                let lx = local.x - b.x;
                let ly = local.y - b.y;
                let pen = CreatePen(PS_SOLID, 3, COLORREF(0x00F0_8020));
                let old_pen = SelectObject(mem_dc, pen.into());
                let old_br = SelectObject(mem_dc, GetStockObject(HOLLOW_BRUSH));
                let _ = Rectangle(mem_dc, lx, ly, lx + local.width, ly + local.height);
                SelectObject(mem_dc, old_br);
                SelectObject(mem_dc, old_pen);
                let _ = DeleteObject(pen.into());

                let chip = CaptureUx::size_chip(hl.width, hl.height);
                let chip_rc = RECT {
                    left: lx.max(4),
                    top: (ly - 26).max(4),
                    right: (lx + 110).min(cw - 4),
                    bottom: (ly - 4).max(26),
                };
                let bg = CreateSolidBrush(COLORREF(0x0018_1818));
                FillRect(mem_dc, &chip_rc, bg);
                let _ = DeleteObject(bg.into());
                let old_f = SelectObject(mem_dc, st.font.into());
                SetBkMode(mem_dc, TRANSPARENT);
                SetTextColor(mem_dc, COLORREF(0x00FF_FFFF));
                let wt: Vec<u16> = chip.encode_utf16().collect();
                let _ = TextOutW(mem_dc, chip_rc.left + 6, chip_rc.top + 3, &wt);
                SelectObject(mem_dc, old_f);
            }
        }

        // Top corner hint
        let hint_rc = RECT {
            left: 16,
            top: 16,
            right: 340,
            bottom: 44,
        };
        let bg = CreateSolidBrush(COLORREF(0x0018_1818));
        FillRect(mem_dc, &hint_rc, bg);
        let _ = DeleteObject(bg.into());
        let old_f = SelectObject(mem_dc, st.font.into());
        SetBkMode(mem_dc, TRANSPARENT);
        SetTextColor(mem_dc, COLORREF(0x00FF_FFFF));
        let wh: Vec<u16> = ScrollCapture::PICK_HINT.encode_utf16().collect();
        let _ = TextOutW(mem_dc, 24, 21, &wh);
        SelectObject(mem_dc, old_f);

        let _ = BitBlt(hdc, 0, 0, cw, ch, Some(mem_dc), 0, 0, SRCCOPY);
        SelectObject(mem_dc, old_bmp);
        let _ = DeleteObject(mem_bmp.into());
        let _ = DeleteDC(mem_dc);
        let _ = EndPaint(hwnd, &ps);
    }
}

unsafe extern "system" fn border_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

unsafe extern "system" fn pill_proc(
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
                let boxed = Box::from_raw(raw as *mut (Arc<AtomicBool>, Arc<AtomicBool>, HBRUSH));
                if !boxed.2.is_invalid() {
                    let _ = DeleteObject(boxed.2.into());
                }
                drop(boxed);
            }
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr != 0 {
        let state = unsafe { &*(ptr as *const (Arc<AtomicBool>, Arc<AtomicBool>, HBRUSH)) };
        match msg {
            WM_ERASEBKGND => {
                let hdc = windows::Win32::Graphics::Gdi::HDC(wparam.0 as *mut _);
                let mut rc = RECT::default();
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rc);
                    FillRect(hdc, &rc, state.2);
                }
                return LRESULT(1);
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
                let hdc = windows::Win32::Graphics::Gdi::HDC(wparam.0 as *mut _);
                unsafe {
                    SetTextColor(hdc, COLORREF(0x00F5_F5F5));
                    SetBkColor(hdc, COLORREF(0x001F_1F1F));
                }
                return LRESULT(state.2.0 as isize);
            }
            windows::Win32::UI::WindowsAndMessaging::WM_COMMAND => {
                let id = wparam.0 & 0xFFFF;
                if id == 1 {
                    state.0.store(true, Ordering::Relaxed);
                } else if id == 2 {
                    state.1.store(true, Ordering::Relaxed);
                }
                return LRESULT(0);
            }
            WM_KEYDOWN => {
                let vk = wparam.0 as u16;
                if vk == VK_RETURN.0 {
                    state.0.store(true, Ordering::Relaxed);
                    return LRESULT(0);
                }
                if vk == VK_ESCAPE.0 {
                    state.1.store(true, Ordering::Relaxed);
                    return LRESULT(0);
                }
            }
            _ => {}
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[allow(dead_code)]
fn _unused_d2d(_: D2D1_COLOR_F, _: fn() -> i32) {
    let _ = MoveToEx;
    let _ = LineTo;
}
