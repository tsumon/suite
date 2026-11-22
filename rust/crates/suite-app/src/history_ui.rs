use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use chrono::Local;
use suite_capture::{PixelBuffer, ScreenshotHistory};
use suite_contracts::AppSettings;
use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, CreateSolidBrush, DIB_RGB_COLORS,
    DeleteObject, EndPaint, FillRect, HFONT, PAINTSTRUCT, SRCCOPY, SelectObject, SetBkMode,
    SetTextColor, StretchDIBits, TRANSPARENT, TextOutW,
};
use windows::Win32::UI::Controls::SetScrollInfo;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, GWLP_USERDATA, HMENU,
    PostMessageW, SB_VERT, SCROLLINFO, SIF_PAGE, SIF_POS, SIF_RANGE, SW_SHOW, SetForegroundWindow,
    SetWindowLongPtrW, ShowWindow, WM_COMMAND, WM_DESTROY, WM_MOUSEWHEEL, WM_NCDESTROY, WM_PAINT,
    WM_VSCROLL, WS_CAPTION, WS_CHILD, WS_OVERLAPPED, WS_SYSMENU, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::PCWSTR;

use crate::messages::{AppCommand, WM_APP_HISTORY_CLOSED, post_boxed, post_command};
use crate::win32_util::{
    confirm_warning, create_ui_font, hinstance, invalidate_all, open_shell_target,
    register_window_class, set_control_font,
};

const HISTORY_WND_CLASS: &str = "SuiteHistoryWnd";
const ROW_H: i32 = 56;
const VISIBLE_ROWS: usize = 5;

struct HistoryRow {
    path: PathBuf,
    time_label: String,
    thumb: Option<PixelBuffer>,
}

struct HistoryState {
    hwnd: HWND,
    controller: HWND,
    settings: AppSettings,
    font: HFONT,
    rows: Vec<HistoryRow>,
    scroll_pos: usize,
    row_buttons: Vec<HWND>,
    path_label: HWND,
    empty_label: HWND,
}

pub struct HistoryWindow {
    state: Rc<RefCell<HistoryState>>,
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
        let font = create_ui_font(13, false, false, "Microsoft YaHei UI");
        let state = Rc::new(RefCell::new(HistoryState {
            hwnd: HWND::default(),
            controller,
            settings: settings.clone(),
            font,
            rows: Vec::new(),
            scroll_pos: 0,
            row_buttons: Vec::new(),
            path_label: HWND::default(),
            empty_label: HWND::default(),
        }));

        let cls = to_wide_null(HISTORY_WND_CLASS);
        let title = to_wide_null("截图历史");
        let raw = Rc::into_raw(Rc::clone(&state));
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(cls.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL,
                160,
                160,
                580,
                400,
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
        build_static_controls(hwnd, &state);
        let win = Rc::new(Self { state });
        win.reload(settings);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
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
            let thumb = ScreenshotHistory::load_png(&e.path).ok();
            rows.push(HistoryRow {
                path: e.path,
                time_label,
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
                let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowTextW(
                    path_lbl,
                    PCWSTR(wdir.as_ptr()),
                );
            }
        }
        rebuild_visible_buttons(&self.state);
        update_scrollbar(&self.state);
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
    let font = state.borrow().font;
    let lbl_cls = to_wide_null("STATIC");
    let btn_cls = to_wide_null("BUTTON");
    unsafe {
        if let Ok(pl) = CreateWindowExW(
            Default::default(),
            PCWSTR(lbl_cls.as_ptr()),
            PCWSTR(to_wide_null("目录：").as_ptr()),
            WS_CHILD | WS_VISIBLE,
            14,
            10,
            520,
            20,
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
            14,
            52,
            240,
            24,
            Some(hwnd),
            None,
            Some(hinstance()),
            None,
        ) {
            set_control_font(el, font);
            state.borrow_mut().empty_label = el;
        }

        let mk_bottom = |label: &str, x: i32, w: i32, id: usize| {
            if let Ok(b) = CreateWindowExW(
                Default::default(),
                PCWSTR(btn_cls.as_ptr()),
                PCWSTR(to_wide_null(label).as_ptr()),
                WS_CHILD | WS_VISIBLE,
                x,
                326,
                w,
                28,
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            ) {
                set_control_font(b, font);
            }
        };
        mk_bottom("打开目录", 14, 90, 9101);
        mk_bottom("清空历史", 112, 90, 9102);
        mk_bottom("关闭", 464, 80, 9103);
    }
}

fn rebuild_visible_buttons(state: &Rc<RefCell<HistoryState>>) {
    let (hwnd, font, old_btns, empty_lbl, total, scroll_pos) = {
        let mut st = state.borrow_mut();
        let old = std::mem::take(&mut st.row_buttons);
        (
            st.hwnd,
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
    for b in old_btns {
        unsafe {
            let _ = DestroyWindow(b);
        }
    }
    if !empty_lbl.is_invalid() {
        unsafe {
            let _ = ShowWindow(
                empty_lbl,
                if total == 0 {
                    SW_SHOW
                } else {
                    windows::Win32::UI::WindowsAndMessaging::SW_HIDE
                },
            );
        }
    }

    let btn_cls = to_wide_null("BUTTON");
    let mut created = Vec::new();
    let end = (scroll_pos + VISIBLE_ROWS).min(total);
    for (slot, row_idx) in (scroll_pos..end).enumerate() {
        let y = 38 + (slot as i32) * ROW_H + 14;
        let mut mk = |label: &str, x: i32, id: usize| {
            let wt = to_wide_null(label);
            unsafe {
                if let Ok(b) = CreateWindowExW(
                    Default::default(),
                    PCWSTR(btn_cls.as_ptr()),
                    PCWSTR(wt.as_ptr()),
                    WS_CHILD | WS_VISIBLE,
                    x,
                    y,
                    58,
                    26,
                    Some(hwnd),
                    Some(HMENU(id as *mut _)),
                    Some(hinstance()),
                    None,
                ) {
                    set_control_font(b, font);
                    created.push(b);
                }
            }
        };
        mk("复制", 350, 1000 + row_idx);
        mk("钉图", 414, 2000 + row_idx);
        mk("删除", 478, 3000 + row_idx);
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
                let font = rc.borrow().font;
                if !font.is_invalid() {
                    let _ = DeleteObject(font.into());
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
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            if !hdc.is_invalid() {
                let st = state_rc.borrow();
                unsafe {
                    let bg = CreateSolidBrush(COLORREF(0x00F5_F5F5));
                    FillRect(hdc, &ps.rcPaint, bg);
                    let _ = DeleteObject(bg.into());
                    let of = SelectObject(hdc, st.font.into());
                    SetBkMode(hdc, TRANSPARENT);
                    SetTextColor(hdc, COLORREF(0x001E_1E1E));

                    let end = (st.scroll_pos + VISIBLE_ROWS).min(st.rows.len());
                    for (slot, row_idx) in (st.scroll_pos..end).enumerate() {
                        let y = 38 + (slot as i32) * ROW_H;
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
                                14,
                                y + 6,
                                68,
                                44,
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
                        let wlbl: Vec<u16> = row.time_label.encode_utf16().collect();
                        let _ = TextOutW(hdc, 92, y + 18, &wlbl);
                    }
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
                    0 => st.scroll_pos = st.scroll_pos.saturating_sub(1), // SB_LINEUP
                    1 => st.scroll_pos = (st.scroll_pos + 1).min(max_pos), // SB_LINEDOWN
                    2 => st.scroll_pos = st.scroll_pos.saturating_sub(VISIBLE_ROWS), // SB_PAGEUP
                    3 => st.scroll_pos = (st.scroll_pos + VISIBLE_ROWS).min(max_pos), // SB_PAGEDOWN
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
        WM_COMMAND => {
            let id = wparam.0 & 0xFFFF;
            let (controller, settings) = {
                let st = state_rc.borrow();
                (st.controller, st.settings.clone())
            };
            if (1000..2000).contains(&id) {
                let idx = id - 1000;
                let path = state_rc.borrow().rows.get(idx).map(|r| r.path.clone());
                if let Some(p) = path {
                    let _ = ScreenshotHistory::try_copy_file_to_clipboard(&p);
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
