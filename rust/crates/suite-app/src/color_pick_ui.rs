use std::cell::RefCell;
use std::rc::Rc;

use suite_capture::{
    ColorPick, ImageClipboard, MonitorCapture, MonitorEnumerator, PixelBuffer, PixelRect,
    ScreenCapturePipeline,
};
use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, CreateCompatibleBitmap,
    CreateCompatibleDC, CreatePen, CreateSolidBrush, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    EndPaint, FillRect, HBRUSH, LineTo, MoveToEx, PAINTSTRUCT, PS_SOLID, Rectangle, SRCCOPY,
    SelectObject, SetBkMode, SetTextColor, StretchDIBits, TRANSPARENT, TextOutW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_ESCAPE, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, GWLP_USERDATA,
    HWND_TOPMOST, IDC_CROSS, LoadCursorW, SW_SHOW, SetForegroundWindow, SetWindowLongPtrW,
    ShowWindow, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MOUSEMOVE,
    WM_NCDESTROY, WM_PAINT, WM_RBUTTONUP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::PCWSTR;

use crate::messages::{ColorPickDoneMsg, WM_APP_COLOR_PICK_DONE, post_boxed};
use crate::win32_util::{create_ui_font, hinstance, invalidate_all, register_window_class};

const COLOR_PICK_WND_CLASS: &str = "SuiteColorPickOverlayWnd";

struct SharedPickSession {
    controller: HWND,
    windows: Vec<HWND>,
    finished: bool,
    cursor_virtual: (i32, i32),
}

struct PickOverlayState {
    session: Rc<RefCell<SharedPickSession>>,
    capture: MonitorCapture,
    font: windows::Win32::Graphics::Gdi::HFONT,
}

pub struct ColorPickSession {
    shared: Rc<RefCell<SharedPickSession>>,
}

impl ColorPickSession {
    pub fn begin(controller: HWND) -> Result<Self, String> {
        let frames = ScreenCapturePipeline::capture_all()?;
        let cross = unsafe { LoadCursorW(None, IDC_CROSS).ok() };
        register_window_class(COLOR_PICK_WND_CLASS, Some(color_pick_wnd_proc), false, cross);

        let cursor_virtual = MonitorEnumerator::cursor_pos();
        let shared = Rc::new(RefCell::new(SharedPickSession {
            controller,
            windows: Vec::with_capacity(frames.len()),
            finished: false,
            cursor_virtual,
        }));

        let cls = to_wide_null(COLOR_PICK_WND_CLASS);
        let title = to_wide_null("Suite ColorPick");
        let mut first_hwnd = HWND::default();

        for cap in frames {
            let b = cap.monitor.bounds;
            let font = create_ui_font(13, false, true, "Consolas");
            let state = Rc::new(RefCell::new(PickOverlayState {
                session: Rc::clone(&shared),
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
            if first_hwnd.is_invalid() || b.contains(cursor_virtual.0, cursor_virtual.1) {
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

        Ok(Self { shared })
    }

    pub fn cancel(&self) {
        close_pick_session(&self.shared, None, None);
    }
}

impl Drop for ColorPickSession {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn close_pick_session(
    shared: &Rc<RefCell<SharedPickSession>>,
    copied_value: Option<String>,
    error: Option<String>,
) {
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
    for hwnd in windows {
        if !hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
    post_boxed(
        controller,
        WM_APP_COLOR_PICK_DONE,
        0,
        ColorPickDoneMsg {
            copied_value,
            error,
        },
    );
}

fn sample_at_cursor(st: &PickOverlayState, vx: i32, vy: i32) -> Option<(u8, u8, u8)> {
    let b = st.capture.monitor.bounds;
    if !b.contains(vx, vy) {
        return None;
    }
    let px = ((vx - b.x) as f64 * st.capture.buffer.width as f64 / b.width.max(1) as f64)
        .round() as i32;
    let py = ((vy - b.y) as f64 * st.capture.buffer.height as f64 / b.height.max(1) as f64)
        .round() as i32;
    st.capture
        .buffer
        .try_get_pixel(px, py)
        .map(|(blue, green, red, _)| (red, green, blue))
}

unsafe extern "system" fn color_pick_wnd_proc(
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
                let rc = Rc::from_raw(raw as *const RefCell<PickOverlayState>);
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
        let r = Rc::from_raw(ptr as *const RefCell<PickOverlayState>);
        let c = Rc::clone(&r);
        std::mem::forget(r);
        c
    };

    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_pick_overlay(hwnd, &state_rc.borrow());
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let session = Rc::clone(&state_rc.borrow().session);
            let windows = {
                let mut s = session.borrow_mut();
                s.cursor_virtual = (vx, vy);
                s.windows.clone()
            };
            for w in windows {
                invalidate_all(w);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (vx, vy) = MonitorEnumerator::cursor_pos();
            let shift_down = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
            let session = Rc::clone(&state_rc.borrow().session);
            let sampled = sample_at_cursor(&state_rc.borrow(), vx, vy);
            if let Some((r, g, b)) = sampled {
                let text = if shift_down {
                    ColorPick::to_rgb(r, g, b)
                } else {
                    ColorPick::to_hex(r, g, b)
                };
                match ImageClipboard::copy_text(&text) {
                    Ok(()) => close_pick_session(&session, Some(text), None),
                    Err(_) => close_pick_session(&session, None, Some(ColorPick::FAIL.to_string())),
                }
            } else {
                close_pick_session(&session, None, Some(ColorPick::FAIL.to_string()));
            }
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            let session = Rc::clone(&state_rc.borrow().session);
            close_pick_session(&session, None, None);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            let vk = wparam.0 as u16;
            let session = Rc::clone(&state_rc.borrow().session);
            if vk == VK_ESCAPE.0 {
                close_pick_session(&session, None, None);
                return LRESULT(0);
            }
            if vk == b'C' as u16 {
                let (vx, vy) = MonitorEnumerator::cursor_pos();
                if let Some((r, g, b)) = sample_at_cursor(&state_rc.borrow(), vx, vy) {
                    let hex = ColorPick::to_hex(r, g, b);
                    match ImageClipboard::copy_text(&hex) {
                        Ok(()) => close_pick_session(&session, Some(hex), None),
                        Err(_) => {
                            close_pick_session(&session, None, Some(ColorPick::FAIL.to_string()))
                        }
                    }
                }
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_DESTROY => LRESULT(0),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn paint_pick_overlay(hwnd: HWND, st: &PickOverlayState) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    if hdc.is_invalid() {
        return;
    }

    let b = st.capture.monitor.bounds;
    let cw = b.width.max(1);
    let ch = b.height.max(1);

    unsafe {
        let mem_dc = CreateCompatibleDC(Some(hdc));
        let mem_bmp = CreateCompatibleBitmap(hdc, cw, ch);
        let old_bmp = SelectObject(mem_dc, mem_bmp.into());

        blit_pixel_buffer(mem_dc, &st.capture.buffer, cw, ch);

        let (vx, vy) = st.session.borrow().cursor_virtual;
        if b.contains(vx, vy) {
            let lx = vx - b.x;
            let ly = vy - b.y;
            draw_magnifier_box(
                mem_dc,
                &st.capture.buffer,
                b,
                lx,
                ly,
                cw,
                ch,
                st.font,
            );
        }

        let _ = BitBlt(hdc, 0, 0, cw, ch, Some(mem_dc), 0, 0, SRCCOPY);
        SelectObject(mem_dc, old_bmp);
        let _ = DeleteObject(mem_bmp.into());
        let _ = DeleteDC(mem_dc);
        let _ = EndPaint(hwnd, &ps);
    }
}

pub fn blit_pixel_buffer(hdc: windows::Win32::Graphics::Gdi::HDC, buf: &PixelBuffer, dst_w: i32, dst_h: i32) {
    if buf.width <= 0 || buf.height <= 0 || buf.bgra.is_empty() {
        return;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: buf.width,
            biHeight: -buf.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    unsafe {
        let _ = StretchDIBits(
            hdc,
            0,
            0,
            dst_w,
            dst_h,
            0,
            0,
            buf.width,
            buf.height,
            Some(buf.bgra.as_ptr().cast()),
            &bmi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_magnifier_box(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    buf: &PixelBuffer,
    mon_bounds: PixelRect,
    lx: i32,
    ly: i32,
    cw: i32,
    ch: i32,
    font: windows::Win32::Graphics::Gdi::HFONT,
) {
    const SRC_HALF: i32 = 7;
    const SCALE: i32 = 8;
    const ZOOM_SZ: i32 = (SRC_HALF * 2 + 1) * SCALE; // 120
    const PANEL_H: i32 = ZOOM_SZ + 44;

    let mut box_x = lx + 20;
    let mut box_y = ly + 20;
    if box_x + ZOOM_SZ + 8 > cw {
        box_x = lx - ZOOM_SZ - 24;
    }
    if box_y + PANEL_H + 8 > ch {
        box_y = ly - PANEL_H - 24;
    }
    box_x = box_x.max(8);
    box_y = box_y.max(8);

    let px = ((lx as f64) * buf.width as f64 / mon_bounds.width.max(1) as f64).round() as i32;
    let py = ((ly as f64) * buf.height as f64 / mon_bounds.height.max(1) as f64).round() as i32;

    unsafe {
        let bg_brush = CreateSolidBrush(COLORREF(0x0020_2020));
        let card = RECT {
            left: box_x - 2,
            top: box_y - 2,
            right: box_x + ZOOM_SZ + 2,
            bottom: box_y + PANEL_H,
        };
        FillRect(hdc, &card, bg_brush);
        let _ = DeleteObject(bg_brush.into());

        // Zoomed pixel grid
        for sy in -SRC_HALF..=SRC_HALF {
            for sx in -SRC_HALF..=SRC_HALF {
                let (b, g, r, _) = buf
                    .try_get_pixel(px + sx, py + sy)
                    .unwrap_or((30, 30, 30, 255));
                let br = CreateSolidBrush(COLORREF(
                    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16),
                ));
                let cell = RECT {
                    left: box_x + (sx + SRC_HALF) * SCALE,
                    top: box_y + (sy + SRC_HALF) * SCALE,
                    right: box_x + (sx + SRC_HALF + 1) * SCALE,
                    bottom: box_y + (sy + SRC_HALF + 1) * SCALE,
                };
                FillRect(hdc, &cell, br);
                let _ = DeleteObject(br.into());
            }
        }

        // Crosshair
        let pen = CreatePen(PS_SOLID, 1, COLORREF(0x00F0_8020));
        let old_pen = SelectObject(hdc, pen.into());
        let old_br = SelectObject(hdc, HBRUSH::default().into());
        let cx = box_x + SRC_HALF * SCALE + SCALE / 2;
        let cy = box_y + SRC_HALF * SCALE + SCALE / 2;
        let _ = MoveToEx(hdc, box_x, cy, None);
        let _ = LineTo(hdc, box_x + ZOOM_SZ, cy);
        let _ = MoveToEx(hdc, cx, box_y, None);
        let _ = LineTo(hdc, cx, box_y + ZOOM_SZ);
        let _ = Rectangle(
            hdc,
            box_x + SRC_HALF * SCALE,
            box_y + SRC_HALF * SCALE,
            box_x + (SRC_HALF + 1) * SCALE,
            box_y + (SRC_HALF + 1) * SCALE,
        );
        SelectObject(hdc, old_br);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen.into());

        let (b, g, r, _) = buf.try_get_pixel(px, py).unwrap_or((0, 0, 0, 255));
        let line1 = format!("{}  ({},{})", ColorPick::to_hex(r, g, b), px, py);
        let line2 = format!("RGB({r},{g},{b})");
        let w1: Vec<u16> = line1.encode_utf16().collect();
        let w2: Vec<u16> = line2.encode_utf16().collect();
        let old_font = SelectObject(hdc, font.into());
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(0x00FF_FFFF));
        let _ = TextOutW(hdc, box_x + 4, box_y + ZOOM_SZ + 4, &w1);
        SetTextColor(hdc, COLORREF(0x00CC_CCCC));
        let _ = TextOutW(hdc, box_x + 4, box_y + ZOOM_SZ + 22, &w2);
        SelectObject(hdc, old_font);
    }
}
