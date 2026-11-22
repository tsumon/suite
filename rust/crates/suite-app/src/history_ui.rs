use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::Local;
use suite_capture::{PixelBuffer, ScreenshotHistory};
use suite_contracts::{AppSettings, ThemeKind};
use suite_platform::{PersonalizeRegistry, constants::to_wide_null};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, CreatePen, CreateSolidBrush, DIB_RGB_COLORS,
    DeleteObject, EndPaint, FillRect, HALFTONE, HBRUSH, HDC, HFONT, LineTo, MoveToEx, PAINTSTRUCT,
    PS_SOLID, SRCCOPY, SelectObject, SetBkColor, SetBkMode, SetStretchBltMode, SetTextColor,
    StretchDIBits, TRANSPARENT, TextOutW,
};
use windows::Win32::UI::Controls::SetScrollInfo;
use windows::Win32::UI::HiDpi::AdjustWindowRectExForDpi;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, GWLP_USERDATA, HMENU,
    PostMessageW, SB_VERT, SCROLLINFO, SIF_PAGE, SIF_POS, SIF_RANGE, SW_HIDE, SW_SHOW,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowTextW, ShowWindow, WINDOW_EX_STYLE,
    WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN,
    WM_MOUSEWHEEL, WM_NCDESTROY, WM_PAINT, WM_VSCROLL, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN,
    WS_OVERLAPPED, WS_SYSMENU, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::PCWSTR;

use crate::messages::{AppCommand, WM_APP_HISTORY_CLOSED, post_boxed, post_command};
use crate::win32_util::{
    apply_control_theme, apply_window_theme, centered_origin_on_cursor_monitor, confirm_warning,
    create_ui_font, dpi_for_cursor, hinstance, invalidate_all, open_shell_target,
    register_window_class, s_px, set_control_font,
};

const HISTORY_WND_CLASS: &str = "SuiteHistoryWnd";
const CLIENT_W_DIP: i32 = 620;
const CLIENT_H_DIP: i32 = 440;
const ROW_H_DIP: i32 = 64;
const VISIBLE_ROWS: usize = 5;
const THUMB_MAX_W: i32 = 160;
const THUMB_MAX_H: i32 = 104;

struct HistoryRow {
    path: PathBuf,
    time_label: String,
    dims_label: String,
    thumb: Option<PixelBuffer>,
}

struct HistoryState {
    hwnd: HWND,
    controller: HWND,
    dpi: u32,
    light: bool,
    bg_brush: HBRUSH,
    settings: AppSettings,
    font: HFONT,
    rows: Vec<HistoryRow>,
    scroll_pos: usize,
    row_buttons: Vec<HWND>,
    themed_controls: Vec<HWND>,
    path_label: HWND,
    empty_label: HWND,
    status_label: HWND,
}

pub struct HistoryWindow {
    state: Rc<RefCell<HistoryState>>,
}

/// Downsamples a full-resolution BGRA buffer into a small preview thumbnail.
pub fn downsample_thumbnail(src: &PixelBuffer, max_w: i32, max_h: i32) -> PixelBuffer {
    if src.width <= 0 || src.height <= 0 || src.bgra.is_empty() || max_w <= 0 || max_h <= 0 {
        return PixelBuffer::allocate(1, 1);
    }
    let scale = (max_w as f64 / src.width as f64)
        .min(max_h as f64 / src.height as f64)
        .min(1.0);
    let dst_w = ((src.width as f64 * scale).round() as i32).max(1);
    let dst_h = ((src.height as f64 * scale).round() as i32).max(1);
    let mut dst = PixelBuffer::allocate(dst_w, dst_h);
    for dy in 0..dst_h {
        let sy = ((dy as i64 * i64::from(src.height)) / i64::from(dst_h))
            .clamp(0, i64::from(src.height - 1)) as usize;
        let srow = sy * (src.stride as usize);
        let drow = (dy as usize) * (dst.stride as usize);
        for dx in 0..dst_w {
            let sx = ((dx as i64 * i64::from(src.width)) / i64::from(dst_w))
                .clamp(0, i64::from(src.width - 1)) as usize;
            let si = srow + sx * 4;
            let di = drow + (dx as usize) * 4;
            dst.bgra[di..di + 4].copy_from_slice(&src.bgra[si..si + 4]);
        }
    }
    dst
}

impl HistoryWindow {
    pub fn show_or_focus(
        existing: Option<&Self>,
        controller: HWND,
        settings: &AppSettings,
    ) -> Rc<Self> {
        if let Some(win) = existing {
            let hwnd = win.state.borrow().hwnd;
            if !hwnd.is_invalid() {
                win.reload(settings);
                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
                return Rc::new(Self {
                    state: Rc::clone(&win.state),
                });
            }
        }

        register_window_class(HISTORY_WND_CLASS, Some(history_wnd_proc), false, None);
        let dpi = dpi_for_cursor();
        let light = PersonalizeRegistry::try_read()
            .map(|p| p.apps_use_light)
            .unwrap_or(true);
        let bg_brush = unsafe {
            CreateSolidBrush(if light {
                COLORREF(0x00F6_F6F6)
            } else {
                COLORREF(0x0024_2424)
            })
        };
        let font = create_ui_font(s_px(13, dpi), false, false, "Microsoft YaHei UI");
        let state = Rc::new(RefCell::new(HistoryState {
            hwnd: HWND::default(),
            controller,
            dpi,
            light,
            bg_brush,
            settings: settings.clone(),
            font,
            rows: Vec::new(),
            scroll_pos: 0,
            row_buttons: Vec::new(),
            themed_controls: Vec::new(),
            path_label: HWND::default(),
            empty_label: HWND::default(),
            status_label: HWND::default(),
        }));

        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL | WS_CLIPCHILDREN;
        let mut adj = RECT {
            left: 0,
            top: 0,
            right: s_px(CLIENT_W_DIP, dpi),
            bottom: s_px(CLIENT_H_DIP, dpi),
        };
        unsafe {
            let _ = AdjustWindowRectExForDpi(&mut adj, style, false, WINDOW_EX_STYLE(0), dpi);
        }
        let win_w = (adj.right - adj.left).max(s_px(CLIENT_W_DIP, dpi));
        let win_h = (adj.bottom - adj.top).max(s_px(CLIENT_H_DIP, dpi));
        let (wx, wy) = centered_origin_on_cursor_monitor(win_w, win_h);

        let cls = to_wide_null(HISTORY_WND_CLASS);
        let title = to_wide_null("截图历史");
        let raw = Rc::into_raw(Rc::clone(&state));
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(cls.as_ptr()),
                PCWSTR(title.as_ptr()),
                style,
                wx,
                wy,
                win_w,
                win_h,
                None,
                None,
                Some(hinstance()),
                Some(raw.cast()),
            )
            .unwrap_or_default()
        };
        if hwnd.is_invalid() {
            unsafe {
                drop(Rc::from_raw(raw));
            }
            return Rc::new(Self { state });
        }

        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        }
        state.borrow_mut().hwnd = hwnd;
        apply_window_theme(hwnd, !light);
        build_static_controls(hwnd, &state);
        let win = Rc::new(Self { state });
        win.reload(settings);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
        win
    }

    pub fn reload(&self, settings: &AppSettings) {
        let dir = ScreenshotHistory::directory_for(settings);
        let entries = ScreenshotHistory::list(settings);
        let mut rows = Vec::with_capacity(entries.len());
        for e in entries {
            let time_label = e
                .modified_utc
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            let (dims_label, thumb) = match ScreenshotHistory::load_png(&e.path) {
                Ok(buf) => {
                    let dims = format!("{}×{}", buf.width, buf.height);
                    (dims, Some(downsample_thumbnail(&buf, THUMB_MAX_W, THUMB_MAX_H)))
                }
                Err(_) => (String::new(), None),
            };
            rows.push(HistoryRow {
                path: e.path,
                time_label,
                dims_label,
                thumb,
            });
        }

        let (hwnd, path_lbl) = {
            let mut st = self.state.borrow_mut();
            st.settings = settings.clone();
            st.rows = rows;
            st.scroll_pos = st.scroll_pos.min(st.rows.len().saturating_sub(VISIBLE_ROWS));
            (st.hwnd, st.path_label)
        };
        if !path_lbl.is_invalid() {
            let wdir = to_wide_null(&format!("目录：{}", dir.display()));
            unsafe {
                let _ = SetWindowTextW(path_lbl, PCWSTR(wdir.as_ptr()));
            }
        }
        rebuild_visible_buttons(&self.state);
        update_scrollbar(&self.state);
        invalidate_all(hwnd);
    }

    pub fn apply_theme(&self, theme: ThemeKind) {
        let light = theme == ThemeKind::Light;
        let (hwnd, ctrls, row_btns) = {
            let mut st = self.state.borrow_mut();
            st.light = light;
            if !st.bg_brush.is_invalid() {
                unsafe {
                    let _ = DeleteObject(st.bg_brush.into());
                }
            }
            st.bg_brush = unsafe {
                CreateSolidBrush(if light {
                    COLORREF(0x00F6_F6F6)
                } else {
                    COLORREF(0x0024_2424)
                })
            };
            (st.hwnd, st.themed_controls.clone(), st.row_buttons.clone())
        };
        apply_window_theme(hwnd, !light);
        for c in ctrls.into_iter().chain(row_btns) {
            apply_control_theme(c, !light);
        }
        invalidate_all(hwnd);
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

fn build_static_controls(hwnd: HWND, state: &Rc<RefCell<HistoryState>>) {
    let (dpi, dark, font) = {
        let st = state.borrow();
        (st.dpi, !st.light, st.font)
    };
    let s = |d: i32| s_px(d, dpi);
    let lbl_cls = to_wide_null("STATIC");
    let btn_cls = to_wide_null("BUTTON");
    let mut themed = Vec::new();
    unsafe {
        if let Ok(pl) = CreateWindowExW(
            Default::default(),
            PCWSTR(lbl_cls.as_ptr()),
            PCWSTR(to_wide_null("目录：").as_ptr()),
            WS_CHILD | WS_VISIBLE,
            s(16),
            s(12),
            s(584),
            s(20),
            Some(hwnd),
            None,
            Some(hinstance()),
            None,
        ) {
            set_control_font(pl, font);
            state.borrow_mut().path_label = pl;
        }

        if let Ok(el) = CreateWindowExW(
            Default::default(),
            PCWSTR(lbl_cls.as_ptr()),
            PCWSTR(to_wide_null(ScreenshotHistory::EMPTY).as_ptr()),
            WS_CHILD | WS_VISIBLE,
            s(16),
            s(60),
            s(280),
            s(24),
            Some(hwnd),
            None,
            Some(hinstance()),
            None,
        ) {
            set_control_font(el, font);
            state.borrow_mut().empty_label = el;
        }

        if let Ok(sl) = CreateWindowExW(
            Default::default(),
            PCWSTR(lbl_cls.as_ptr()),
            PCWSTR(to_wide_null("").as_ptr()),
            WS_CHILD | WS_VISIBLE,
            s(226),
            s(398),
            s(260),
            s(22),
            Some(hwnd),
            None,
            Some(hinstance()),
            None,
        ) {
            set_control_font(sl, font);
            state.borrow_mut().status_label = sl;
        }

        let mut mk_bottom = |label: &str, x: i32, w: i32, id: usize| {
            if let Ok(b) = CreateWindowExW(
                Default::default(),
                PCWSTR(btn_cls.as_ptr()),
                PCWSTR(to_wide_null(label).as_ptr()),
                WS_CHILD | WS_VISIBLE,
                s(x),
                s(392),
                s(w),
                s(30),
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            ) {
                set_control_font(b, font);
                apply_control_theme(b, dark);
                themed.push(b);
            }
        };
        mk_bottom("打开目录", 16, 94, 9101);
        mk_bottom("清空历史", 118, 94, 9102);
        mk_bottom("关闭", 514, 86, 9103);
    }
    state.borrow_mut().themed_controls = themed;
}

fn rebuild_visible_buttons(state: &Rc<RefCell<HistoryState>>) {
    let (hwnd, dpi, dark, font, old_btns, empty_lbl, total, scroll_pos) = {
        let mut st = state.borrow_mut();
        let old = std::mem::take(&mut st.row_buttons);
        (
            st.hwnd,
            st.dpi,
            !st.light,
            st.font,
            old,
            st.empty_label,
            st.rows.len(),
            st.scroll_pos,
        )
    };
    if hwnd.is_invalid() {
        return;
    }
    let s = |d: i32| s_px(d, dpi);
    for b in old_btns {
        unsafe {
            let _ = DestroyWindow(b);
        }
    }
    if !empty_lbl.is_invalid() {
        unsafe {
            let _ = ShowWindow(empty_lbl, if total == 0 { SW_SHOW } else { SW_HIDE });
        }
    }

    let btn_cls = to_wide_null("BUTTON");
    let mut created = Vec::new();
    let end = (scroll_pos + VISIBLE_ROWS).min(total);
    for (slot, row_idx) in (scroll_pos..end).enumerate() {
        let y = 44 + (slot as i32) * ROW_H_DIP + 16;
        let mut mk = |label: &str, x: i32, id: usize| {
            let wt = to_wide_null(label);
            unsafe {
                if let Ok(b) = CreateWindowExW(
                    Default::default(),
                    PCWSTR(btn_cls.as_ptr()),
                    PCWSTR(wt.as_ptr()),
                    WS_CHILD | WS_VISIBLE,
                    s(x),
                    s(y),
                    s(60),
                    s(28),
                    Some(hwnd),
                    Some(HMENU(id as *mut _)),
                    Some(hinstance()),
                    None,
                ) {
                    set_control_font(b, font);
                    apply_control_theme(b, dark);
                    created.push(b);
                }
            }
        };
        mk("复制", 390, 1000 + row_idx);
        mk("钉图", 458, 2000 + row_idx);
        mk("删除", 526, 3000 + row_idx);
    }
    state.borrow_mut().row_buttons = created;
}

fn update_scrollbar(state: &Rc<RefCell<HistoryState>>) {
    let (hwnd, total, pos) = {
        let st = state.borrow();
        (st.hwnd, st.rows.len(), st.scroll_pos)
    };
    if hwnd.is_invalid() {
        return;
    }
    let si = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        nMin: 0,
        nMax: total.saturating_sub(1) as i32,
        nPage: VISIBLE_ROWS as u32,
        nPos: pos as i32,
        ..Default::default()
    };
    unsafe {
        SetScrollInfo(hwnd, SB_VERT, &si, true);
    }
}

unsafe extern "system" fn history_wnd_proc(
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
                let rc = Rc::from_raw(raw as *const RefCell<HistoryState>);
                {
                    let st = rc.borrow();
                    if !st.bg_brush.is_invalid() {
                        let _ = DeleteObject(st.bg_brush.into());
                    }
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
        let r = Rc::from_raw(ptr as *const RefCell<HistoryState>);
        let c = Rc::clone(&r);
        std::mem::forget(r);
        c
    };

    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let hdc = HDC(wparam.0 as *mut _);
            let st = state_rc.borrow();
            let (fg, bg) = if st.light {
                (COLORREF(0x0017_1717), COLORREF(0x00F6_F6F6))
            } else {
                (COLORREF(0x00F5_F5F5), COLORREF(0x0024_2424))
            };
            unsafe {
                SetTextColor(hdc, fg);
                SetBkColor(hdc, bg);
            }
            LRESULT(st.bg_brush.0 as isize)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            if !hdc.is_invalid() {
                let st = state_rc.borrow();
                let s = |d: i32| s_px(d, st.dpi);
                let (fg, muted, line_col) = if st.light {
                    (
                        COLORREF(0x0017_1717),
                        COLORREF(0x0060_6060),
                        COLORREF(0x00DC_DCDC),
                    )
                } else {
                    (
                        COLORREF(0x00F5_F5F5),
                        COLORREF(0x00A8_A8A8),
                        COLORREF(0x0038_3838),
                    )
                };
                unsafe {
                    FillRect(hdc, &ps.rcPaint, st.bg_brush);
                    let of = SelectObject(hdc, st.font.into());
                    SetBkMode(hdc, TRANSPARENT);
                    let _ = SetStretchBltMode(hdc, HALFTONE);

                    let pen = CreatePen(PS_SOLID, 1, line_col);
                    let old_p = SelectObject(hdc, pen.into());
                    let _ = MoveToEx(hdc, s(16), s(38), None);
                    let _ = LineTo(hdc, s(600), s(38));

                    let end = (st.scroll_pos + VISIBLE_ROWS).min(st.rows.len());
                    for (slot, row_idx) in (st.scroll_pos..end).enumerate() {
                        let y = s(44 + (slot as i32) * ROW_H_DIP);
                        let row = &st.rows[row_idx];
                        if let Some(ref thumb) = row.thumb {
                            let bmi = BITMAPINFO {
                                bmiHeader: BITMAPINFOHEADER {
                                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                                    biWidth: thumb.width,
                                    biHeight: -thumb.height,
                                    biPlanes: 1,
                                    biBitCount: 32,
                                    biCompression: BI_RGB.0,
                                    ..Default::default()
                                },
                                ..Default::default()
                            };
                            let _ = StretchDIBits(
                                hdc,
                                s(16),
                                y + s(6),
                                s(80),
                                s(52),
                                0,
                                0,
                                thumb.width,
                                thumb.height,
                                Some(thumb.bgra.as_ptr().cast()),
                                &bmi,
                                DIB_RGB_COLORS,
                                SRCCOPY,
                            );
                        }
                        SetTextColor(hdc, fg);
                        let wlbl: Vec<u16> = row.time_label.encode_utf16().collect();
                        let _ = TextOutW(hdc, s(108), y + s(12), &wlbl);
                        if !row.dims_label.is_empty() {
                            SetTextColor(hdc, muted);
                            let wdim: Vec<u16> = row.dims_label.encode_utf16().collect();
                            let _ = TextOutW(hdc, s(108), y + s(32), &wdim);
                        }
                        let _ = MoveToEx(hdc, s(16), y + s(ROW_H_DIP) - 1, None);
                        let _ = LineTo(hdc, s(600), y + s(ROW_H_DIP) - 1);
                    }
                    SelectObject(hdc, old_p);
                    let _ = DeleteObject(pen.into());
                    SelectObject(hdc, of);
                    let _ = EndPaint(hwnd, &ps);
                }
            }
            LRESULT(0)
        }
        WM_VSCROLL => {
            let code = (wparam.0 & 0xFFFF) as u32;
            {
                let mut st = state_rc.borrow_mut();
                let max_pos = st.rows.len().saturating_sub(VISIBLE_ROWS);
                match code {
                    0 => st.scroll_pos = st.scroll_pos.saturating_sub(1),
                    1 => st.scroll_pos = (st.scroll_pos + 1).min(max_pos),
                    2 => st.scroll_pos = st.scroll_pos.saturating_sub(VISIBLE_ROWS),
                    3 => st.scroll_pos = (st.scroll_pos + VISIBLE_ROWS).min(max_pos),
                    5 => {
                        let track = (wparam.0 >> 16) & 0xFFFF;
                        st.scroll_pos = track.min(max_pos);
                    }
                    _ => {}
                }
            }
            rebuild_visible_buttons(&state_rc);
            update_scrollbar(&state_rc);
            invalidate_all(hwnd);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as u16) as i16;
            {
                let mut st = state_rc.borrow_mut();
                let max_pos = st.rows.len().saturating_sub(VISIBLE_ROWS);
                if delta > 0 {
                    st.scroll_pos = st.scroll_pos.saturating_sub(1);
                } else {
                    st.scroll_pos = (st.scroll_pos + 1).min(max_pos);
                }
            }
            rebuild_visible_buttons(&state_rc);
            update_scrollbar(&state_rc);
            invalidate_all(hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 as u16 == VK_ESCAPE.0 {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_COMMAND => {
            let id = wparam.0 & 0xFFFF;
            let (controller, settings, status_lbl) = {
                let st = state_rc.borrow();
                (st.controller, st.settings.clone(), st.status_label)
            };
            if (1000..2000).contains(&id) {
                let idx = id - 1000;
                let path = state_rc.borrow().rows.get(idx).map(|r| r.path.clone());
                if let Some(p) = path {
                    let msg_txt = if ScreenshotHistory::try_copy_file_to_clipboard(&p).is_ok() {
                        ScreenshotHistory::COPIED
                    } else {
                        ScreenshotHistory::MISSING
                    };
                    if !status_lbl.is_invalid() {
                        let w = to_wide_null(msg_txt);
                        unsafe {
                            let _ = SetWindowTextW(status_lbl, PCWSTR(w.as_ptr()));
                        }
                    }
                }
            } else if (2000..3000).contains(&id) {
                let idx = id - 2000;
                let path = state_rc.borrow().rows.get(idx).map(|r| r.path.clone());
                if let Some(p) = path {
                    post_boxed(
                        controller,
                        crate::messages::WM_APP_COMMAND,
                        AppCommand::PinHistoryPath as usize,
                        p.to_string_lossy().into_owned(),
                    );
                }
            } else if (3000..4000).contains(&id) {
                let idx = id - 3000;
                let path = state_rc.borrow().rows.get(idx).map(|r| r.path.clone());
                if let Some(p) = path {
                    ScreenshotHistory::try_delete(&p);
                    let win = HistoryWindow {
                        state: Rc::clone(&state_rc),
                    };
                    win.reload(&settings);
                }
            } else if id == 9101 {
                let dir = ScreenshotHistory::directory_for(&settings);
                let _ = std::fs::create_dir_all(&dir);
                let _ = open_shell_target(&dir.to_string_lossy());
            } else if id == 9102 {
                if confirm_warning(hwnd, "截图历史", "清空截图历史？") {
                    ScreenshotHistory::clear_all(&settings);
                    let win = HistoryWindow {
                        state: Rc::clone(&state_rc),
                    };
                    win.reload(&settings);
                }
            } else if id == 9103 {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let controller = state_rc.borrow().controller;
            unsafe {
                let _ = PostMessageW(Some(controller), WM_APP_HISTORY_CLOSED, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

#[allow(dead_code)]
fn _unused_hmenu(_: HMENU, _: &str) {
    let _ = post_command;
}

#[cfg(test)]
mod tests {
    use super::downsample_thumbnail;
    use suite_capture::PixelBuffer;

    #[test]
    fn downsample_thumbnail_preserves_aspect_and_bounds() {
        let mut src = PixelBuffer::allocate(1920, 1080);
        src.bgra[0] = 0xAA;
        src.bgra[1] = 0xBB;
        src.bgra[2] = 0xCC;
        src.bgra[3] = 0xFF;
        let thumb = downsample_thumbnail(&src, 160, 104);
        assert!(thumb.width <= 160);
        assert!(thumb.height <= 104);
        assert_eq!(thumb.width, 160);
        assert_eq!(thumb.height, 90);
        assert_eq!(thumb.try_get_pixel(0, 0), Some((0xAA, 0xBB, 0xCC, 0xFF)));
    }
}
