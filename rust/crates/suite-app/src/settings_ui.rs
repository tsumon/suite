use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use suite_contracts::{
    AppSettings, HotkeyBinding, HotkeyCopy, NetSpeedCopy, NetSpeedSettings, SettingsPaths,
    TaskbarAppearanceModes, TaskbarFxCopy, TaskbarFxSettings, ThemeKind, UpdateSettings,
};
use suite_netspeed::InterfaceSnapshot;
use suite_platform::{PersonalizeRegistry, SuiteLog, constants::to_wide_null};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    CreatePen, CreateSolidBrush, DIB_RGB_COLORS, DT_CENTER, DT_LEFT, DT_SINGLELINE, DT_VCENTER,
    DeleteDC, DeleteObject, DrawTextW, HBRUSH, HDC, HFONT, LineTo, MoveToEx, PAINTSTRUCT,
    PS_SOLID, SRCCOPY, SelectObject, SetBkColor, SetBkMode, SetDIBitsToDevice, SetTextColor,
    TRANSPARENT, BeginPaint, EndPaint,
};
use windows::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ODT_BUTTON, ODS_DISABLED, ODS_FOCUS, ODS_SELECTED, TBM_SETPOS, TBM_SETRANGE,
    TBS_AUTOTICKS,
};
use windows::Win32::UI::HiDpi::AdjustWindowRectExForDpi;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_ESCAPE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, BS_OWNERDRAW, CB_ADDSTRING, CB_GETCURSEL,
    CB_RESETCONTENT, CB_SETCURSEL, CBN_SELCHANGE, CBS_DROPDOWNLIST, CreateWindowExW,
    DefWindowProcW, DestroyWindow, EN_KILLFOCUS, ES_AUTOHSCROLL, GetWindowLongPtrW,
    GetWindowTextW, GWLP_USERDATA, HMENU, PostMessageW, SW_HIDE, SW_SHOW, SendMessageW,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowTextW, ShowWindow, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX,
    WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND, WM_HSCROLL, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_NCDESTROY, WM_PAINT, WS_BORDER, WS_CAPTION, WS_CHILD,
    WS_CLIPCHILDREN, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::PCWSTR;

use crate::d2d::D2dRenderer;
use crate::messages::{
    AppCommand, WM_APP_SETTINGS_APPLY, WM_APP_SETTINGS_CLOSED, post_boxed, post_command,
};
use crate::update_checker::UpdateChecker;
use crate::win32_util::{
    apply_control_theme, apply_window_theme, centered_origin_on_cursor_monitor,
    choose_color_dialog, client_rect, confirm_warning, create_ui_font, current_working_set_mb,
    dpi_for_cursor, draw_filled_rect, draw_rounded_rect, hinstance, invalidate_all,
    open_shell_target, pick_folder_dialog, register_window_class, s_px, set_control_font,
};

const SETTINGS_WND_CLASS: &str = "SuiteSettingsWnd";
const TBM_GETPOS: u32 = 0x0400;
const WM_MOUSELEAVE: u32 = 0x02A3;

const BST_CHECKED: usize = 1;
const BST_UNCHECKED: usize = 0;

const CLIENT_W_DIP: i32 = 760;
const CLIENT_H_DIP: i32 = 560;
const NAV_W_DIP: i32 = 208;
const FOOTER_H_DIP: i32 = 52;

const NAV_LABELS: [&str; 5] = [
    "常规",
    "截图与热键",
    "任务栏网速",
    "任务栏透明",
    "高级与关于",
];

// Footer buttons
const ID_BTN_RESTORE: usize = 201;
const ID_BTN_CLOSE: usize = 202;

// Page 0: General
const ID_CHK_RUN_STARTUP: usize = 301;
const ID_BTN_TOGGLE_THEME: usize = 302;
const ID_BTN_QUICK_CAPTURE: usize = 303;
const ID_BTN_QUICK_PICK: usize = 304;
const ID_BTN_QUICK_SCROLL: usize = 305;
const ID_BTN_QUICK_PINS: usize = 306;
const ID_BTN_QUICK_DELAYED: usize = 307;

// Page 1: Capture & Hotkeys
const ID_CHK_CTRL: usize = 401;
const ID_CHK_SHIFT: usize = 402;
const ID_CHK_ALT: usize = 403;
const ID_CHK_WIN: usize = 404;
const ID_CMB_CAP_KEY: usize = 405;
const ID_BTN_TRY_NEXT_KEY: usize = 406;
const ID_CMB_PICK_KEY: usize = 407;
const ID_CMB_SCROLL_KEY: usize = 408;
const ID_BTN_START_SCROLL: usize = 409;
const ID_CHK_MAGNIFIER: usize = 410;
const ID_CHK_SAVE_FILE: usize = 411;
const ID_EDIT_SAVE_DIR: usize = 412;
const ID_CHK_PIN_AFTER: usize = 413;
const ID_CHK_HISTORY: usize = 414;
const ID_BTN_OPEN_HISTORY: usize = 415;
const ID_CMB_CLICK_THROUGH: usize = 416;
const ID_CMB_HIST_MAX: usize = 417;
const ID_BTN_BROWSE_DIR: usize = 418;
const ID_BTN_OPEN_DIR: usize = 419;

// Page 2: NetSpeed
const ID_CHK_NET_VISIBLE: usize = 501;
const ID_CHK_NET_EMBED: usize = 502;
const ID_CHK_NET_SEC: usize = 503;
const ID_CMB_ADAPTER: usize = 504;
const ID_BTN_REFRESH_ADAPTER: usize = 505;
const ID_SLD_FONT: usize = 506;
const ID_CHK_SHOW_DOWN: usize = 507;
const ID_CHK_SHOW_UP: usize = 508;
const ID_CHK_TWO_LINE: usize = 509;
const ID_CHK_BOLD: usize = 510;
const ID_CHK_BG_TRANS: usize = 511;
const ID_BTN_BG_COLOR: usize = 512;
const ID_CHK_BORDER: usize = 513;
const ID_BTN_DOWN_COLOR: usize = 514;
const ID_BTN_DOWN_RESET: usize = 515;
const ID_BTN_UP_COLOR: usize = 516;
const ID_BTN_UP_RESET: usize = 517;

// Page 3: TaskbarFx
const ID_CHK_FX_ENABLED: usize = 601;
const ID_CMB_FX_MODE: usize = 602;
const ID_SLD_FX_ALPHA: usize = 603;
const ID_EDIT_FX_ARGB: usize = 604;
const ID_BTN_FX_COLOR: usize = 605;
const ID_CHK_FX_KEEP: usize = 606;
const ID_BTN_USE_ACRYLIC: usize = 607;

// Page 4: Advanced & About
const ID_CHK_LOGGING: usize = 701;
const ID_BTN_OPEN_LOGS: usize = 702;
const ID_CHK_FX_AUTOOFF: usize = 703;
const ID_CMB_CHANNEL: usize = 704;
const ID_BTN_CHECK_UPDATE: usize = 705;
const ID_BTN_OPEN_RELEASE: usize = 706;
const ID_BTN_GITHUB: usize = 707;

struct ThemePalette {
    nav_bg: COLORREF,
    body_bg: COLORREF,
    footer_bg: COLORREF,
    pill_active: COLORREF,
    pill_hover: COLORREF,
    accent: COLORREF,
    line: COLORREF,
    ink: COLORREF,
    ink_secondary: COLORREF,
    ink_muted: COLORREF,
    edit_bg: COLORREF,
    btn_bg: COLORREF,
    btn_pressed: COLORREF,
    btn_border: COLORREF,
    err_ink: COLORREF,
}

fn palette(light: bool) -> ThemePalette {
    if light {
        ThemePalette {
            nav_bg: COLORREF(0x00E8_E8E8),
            body_bg: COLORREF(0x00FA_FAFA),
            footer_bg: COLORREF(0x00F0_F0F0),
            pill_active: COLORREF(0x00FF_FFFF),
            pill_hover: COLORREF(0x00DC_DCDC),
            accent: COLORREF(0x00C0_6700), // RGB #0067C0
            line: COLORREF(0x00D8_D8D8),
            ink: COLORREF(0x0017_1717),
            ink_secondary: COLORREF(0x003A_3A3A),
            ink_muted: COLORREF(0x005A_5A5A),
            edit_bg: COLORREF(0x00FF_FFFF),
            btn_bg: COLORREF(0x00FF_FFFF),
            btn_pressed: COLORREF(0x00E2_E2E2),
            btn_border: COLORREF(0x00CC_CCCC),
            err_ink: COLORREF(0x002B_2BC4),
        }
    } else {
        ThemePalette {
            nav_bg: COLORREF(0x001F_1F1F),
            body_bg: COLORREF(0x002B_2B2B),
            footer_bg: COLORREF(0x0025_2525),
            pill_active: COLORREF(0x003A_3A3A),
            pill_hover: COLORREF(0x002A_2A2A),
            accent: COLORREF(0x00FF_C24C), // RGB #4CC2FF
            line: COLORREF(0x003C_3C3C),
            ink: COLORREF(0x00F5_F5F5),
            ink_secondary: COLORREF(0x00D4_D4D4),
            ink_muted: COLORREF(0x00B0_B0B0),
            edit_bg: COLORREF(0x0038_3838),
            btn_bg: COLORREF(0x0036_3636),
            btn_pressed: COLORREF(0x002E_2E2E),
            btn_border: COLORREF(0x004D_4D4D),
            err_ink: COLORREF(0x0080_8AFF), // RGB #FF8A80
        }
    }
}

struct AdapterEntry {
    if_index: u32,
    alias: String,
    display: String,
}

struct SettingsState {
    hwnd: HWND,
    controller: HWND,
    d2d: Rc<RefCell<D2dRenderer>>,
    dpi: u32,
    loading: bool,
    light: bool,
    bg_brush: HBRUSH,
    edit_brush: HBRUSH,
    font_body: HFONT,
    font_title: HFONT,
    font_section: HFONT,
    font_small: HFONT,
    font_nav_hdr: HFONT,

    current_nav: usize,
    hover_nav: Option<usize>,
    tracking_leave: bool,
    pages: [Vec<HWND>; 5],
    themed_controls: Vec<HWND>,
    monitor_count: usize,
    last_start_label: String,
    download_url: Option<String>,

    status_lbl: HWND,
    fx_status_lbl: HWND,
    hotkey_conflict_lbl: HWND,
    try_next_btn: HWND,
    light_tip_lbl: HWND,
    use_acrylic_btn: HWND,
    btn_open_release: HWND,

    // General controls
    chk_run_startup: HWND,
    lbl_general_summary: HWND,

    // Capture controls
    chk_ctrl: HWND,
    chk_shift: HWND,
    chk_alt: HWND,
    chk_win: HWND,
    cmb_cap_key: HWND,
    cmb_pick_key: HWND,
    cmb_scroll_key: HWND,
    cmb_click_through: HWND,
    chk_magnifier: HWND,
    chk_save_file: HWND,
    edit_save_dir: HWND,
    chk_pin_after: HWND,
    chk_history: HWND,
    cmb_hist_max: HWND,

    // NetSpeed controls
    chk_net_vis: HWND,
    chk_net_embed: HWND,
    chk_net_sec: HWND,
    cmb_adapter: HWND,
    sld_font: HWND,
    lbl_font_val: HWND,
    chk_show_down: HWND,
    chk_show_up: HWND,
    chk_two_line: HWND,
    chk_bold: HWND,
    chk_bg_trans: HWND,
    chk_border: HWND,
    btn_bg_color: HWND,
    btn_down_color: HWND,
    btn_up_color: HWND,
    down_argb: Option<u32>,
    up_argb: Option<u32>,
    bg_argb: Option<u32>,

    // TaskbarFx controls
    chk_fx_enabled: HWND,
    cmb_fx_mode: HWND,
    lbl_fx_mode_help: HWND,
    sld_fx_alpha: HWND,
    lbl_fx_alpha: HWND,
    edit_fx_argb: HWND,
    btn_fx_color: HWND,
    chk_fx_keep: HWND,

    // Advanced controls
    chk_logging: HWND,
    chk_fx_autooff: HWND,
    lbl_last_start: HWND,
    cmb_channel: HWND,

    settings_snapshot: AppSettings,
    adapters: Vec<AdapterEntry>,
    cap_keys: Vec<(i32, String)>,
    fn_keys: Vec<(i32, String)>,
    scroll_keys: Vec<(i32, String)>,
    ct_presets: Vec<(String, HotkeyBinding)>,
    hist_max_options: Vec<i32>,
}

pub struct SettingsWindow {
    state: Rc<RefCell<SettingsState>>,
}

impl SettingsWindow {
    pub fn show_or_focus(
        existing: Option<&Self>,
        controller: HWND,
        d2d: Rc<RefCell<D2dRenderer>>,
        settings: &AppSettings,
        adapters: &[InterfaceSnapshot],
        last_start_label: &str,
        monitor_count: usize,
    ) -> Rc<Self> {
        if let Some(win) = existing {
            let hwnd = win.state.borrow().hwnd;
            if !hwnd.is_invalid() {
                win.state.borrow_mut().monitor_count = monitor_count;
                win.reload(settings, adapters, last_start_label);
                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
                return Rc::new(Self {
                    state: Rc::clone(&win.state),
                });
            }
        }

        register_window_class(SETTINGS_WND_CLASS, Some(settings_wnd_proc), false, None);
        let dpi = dpi_for_cursor();
        let light = PersonalizeRegistry::try_read()
            .map(|p| p.apps_use_light)
            .unwrap_or(true);
        let pal = palette(light);
        let bg_brush = unsafe { CreateSolidBrush(pal.body_bg) };
        let edit_brush = unsafe { CreateSolidBrush(pal.edit_bg) };

        let font_body = create_ui_font(s_px(13, dpi), false, false, "Microsoft YaHei UI");
        let font_title = create_ui_font(s_px(18, dpi), false, true, "Microsoft YaHei UI");
        let font_section = create_ui_font(s_px(12, dpi), false, true, "Microsoft YaHei UI");
        let font_small = create_ui_font(s_px(12, dpi), false, false, "Microsoft YaHei UI");
        let font_nav_hdr = create_ui_font(s_px(11, dpi), false, true, "Microsoft YaHei UI");

        let mut cap_keys = Vec::new();
        for i in 1..=24 {
            cap_keys.push((0x6F + i, format!("F{i}")));
        }
        for c in b'A'..=b'Z' {
            cap_keys.push((i32::from(c), (c as char).to_string()));
        }
        let mut fn_keys = Vec::new();
        for i in 1..=24 {
            fn_keys.push((0x6F + i, format!("F{i}")));
        }
        let mut scroll_keys = vec![(0, "关闭（无热键）".to_string())];
        scroll_keys.extend(fn_keys.clone());

        let ct_presets = default_click_through_presets(&settings.capture.pin_click_through_hotkey);

        let state = Rc::new(RefCell::new(SettingsState {
            hwnd: HWND::default(),
            controller,
            d2d,
            dpi,
            loading: true,
            light,
            bg_brush,
            edit_brush,
            font_body,
            font_title,
            font_section,
            font_small,
            font_nav_hdr,
            current_nav: 0,
            hover_nav: None,
            tracking_leave: false,
            pages: Default::default(),
            themed_controls: Vec::new(),
            monitor_count,
            last_start_label: last_start_label.to_string(),
            download_url: None,
            status_lbl: HWND::default(),
            fx_status_lbl: HWND::default(),
            hotkey_conflict_lbl: HWND::default(),
            try_next_btn: HWND::default(),
            light_tip_lbl: HWND::default(),
            use_acrylic_btn: HWND::default(),
            btn_open_release: HWND::default(),
            chk_run_startup: HWND::default(),
            lbl_general_summary: HWND::default(),
            chk_ctrl: HWND::default(),
            chk_shift: HWND::default(),
            chk_alt: HWND::default(),
            chk_win: HWND::default(),
            cmb_cap_key: HWND::default(),
            cmb_pick_key: HWND::default(),
            cmb_scroll_key: HWND::default(),
            cmb_click_through: HWND::default(),
            chk_magnifier: HWND::default(),
            chk_save_file: HWND::default(),
            edit_save_dir: HWND::default(),
            chk_pin_after: HWND::default(),
            chk_history: HWND::default(),
            cmb_hist_max: HWND::default(),
            chk_net_vis: HWND::default(),
            chk_net_embed: HWND::default(),
            chk_net_sec: HWND::default(),
            cmb_adapter: HWND::default(),
            sld_font: HWND::default(),
            lbl_font_val: HWND::default(),
            chk_show_down: HWND::default(),
            chk_show_up: HWND::default(),
            chk_two_line: HWND::default(),
            chk_bold: HWND::default(),
            chk_bg_trans: HWND::default(),
            chk_border: HWND::default(),
            btn_bg_color: HWND::default(),
            btn_down_color: HWND::default(),
            btn_up_color: HWND::default(),
            down_argb: settings.net_speed.down_color_argb,
            up_argb: settings.net_speed.up_color_argb,
            bg_argb: settings.net_speed.background_argb,
            chk_fx_enabled: HWND::default(),
            cmb_fx_mode: HWND::default(),
            lbl_fx_mode_help: HWND::default(),
            sld_fx_alpha: HWND::default(),
            lbl_fx_alpha: HWND::default(),
            edit_fx_argb: HWND::default(),
            btn_fx_color: HWND::default(),
            chk_fx_keep: HWND::default(),
            chk_logging: HWND::default(),
            chk_fx_autooff: HWND::default(),
            lbl_last_start: HWND::default(),
            cmb_channel: HWND::default(),
            settings_snapshot: settings.clone(),
            adapters: Vec::new(),
            cap_keys,
            fn_keys,
            scroll_keys,
            ct_presets,
            hist_max_options: vec![10, 20, 30, 50, 100, 200],
        }));

        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN;
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

        let cls = to_wide_null(SETTINGS_WND_CLASS);
        let title = to_wide_null(&format!("Suite 设置 — v{}", UpdateChecker::LOCAL_VERSION));
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
        build_settings_controls(hwnd, &state);
        let win = Rc::new(Self { state });
        win.reload(settings, adapters, last_start_label);
        select_nav(&win.state, 0);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
        win
    }

    pub fn reload(
        &self,
        settings: &AppSettings,
        adapters: &[InterfaceSnapshot],
        last_start_label: &str,
    ) {
        {
            let mut st = self.state.borrow_mut();
            st.loading = true;
            st.last_start_label = last_start_label.to_string();
            st.settings_snapshot = settings.clone();
            st.down_argb = settings.net_speed.down_color_argb;
            st.up_argb = settings.net_speed.up_color_argb;
            st.bg_argb = settings.net_speed.background_argb;
            st.adapters = adapters
                .iter()
                .map(|a| AdapterEntry {
                    if_index: a.if_index,
                    alias: a.alias.clone(),
                    display: format!(
                        "{} (IfIndex {}){}",
                        a.alias,
                        a.if_index,
                        if a.is_up { "" } else { " [down]" }
                    ),
                })
                .collect();
        }
        populate_controls_from_state(&self.state, last_start_label);
        refresh_light_tip(&self.state);
        let hwnd = {
            let mut st = self.state.borrow_mut();
            st.loading = false;
            st.hwnd
        };
        invalidate_all(hwnd);
    }

    pub fn apply_theme(&self, theme: ThemeKind) {
        let light = theme == ThemeKind::Light;
        let pal = palette(light);
        let (hwnd, ctrls) = {
            let mut st = self.state.borrow_mut();
            st.light = light;
            if !st.bg_brush.is_invalid() {
                unsafe {
                    let _ = DeleteObject(st.bg_brush.into());
                }
            }
            if !st.edit_brush.is_invalid() {
                unsafe {
                    let _ = DeleteObject(st.edit_brush.into());
                }
            }
            st.bg_brush = unsafe { CreateSolidBrush(pal.body_bg) };
            st.edit_brush = unsafe { CreateSolidBrush(pal.edit_bg) };
            (st.hwnd, st.themed_controls.clone())
        };
        apply_window_theme(hwnd, !light);
        for c in ctrls {
            apply_control_theme(c, !light);
        }
        refresh_light_tip(&self.state);
        invalidate_all(hwnd);
    }

    pub fn set_status(&self, msg: &str) {
        let lbl = self.state.borrow().status_lbl;
        set_text(lbl, msg);
    }

    pub fn set_update_result(&self, msg: &str, download_url: Option<&str>) {
        let (lbl, btn) = {
            let mut st = self.state.borrow_mut();
            st.download_url = download_url.map(ToString::to_string);
            (st.status_lbl, st.btn_open_release)
        };
        set_text(lbl, msg);
        let show = download_url.is_some() && self.state.borrow().current_nav == 4;
        if !btn.is_invalid() {
            unsafe {
                let _ = ShowWindow(btn, if show { SW_SHOW } else { SW_HIDE });
            }
        }
    }

    pub fn set_taskbar_status(&self, msg: &str) {
        let (lbl, slbl) = {
            let st = self.state.borrow();
            (st.fx_status_lbl, st.status_lbl)
        };
        set_text(lbl, msg);
        if msg.contains("无法") || msg.contains("拦住") || msg.contains("暂时找不到") {
            set_text(slbl, msg);
        }
    }

    pub fn set_hotkey_conflict(&self, msg: &str) {
        let (lbl, btn, slbl, show_now) = {
            let st = self.state.borrow();
            (
                st.hotkey_conflict_lbl,
                st.try_next_btn,
                st.status_lbl,
                st.current_nav == 1,
            )
        };
        set_text(lbl, msg);
        set_text(slbl, msg);
        if show_now {
            unsafe {
                let _ = ShowWindow(lbl, SW_SHOW);
                let _ = ShowWindow(btn, SW_SHOW);
            }
        }
    }

    pub fn clear_hotkey_conflict(&self) {
        let (lbl, btn) = {
            let st = self.state.borrow();
            (st.hotkey_conflict_lbl, st.try_next_btn)
        };
        unsafe {
            let _ = ShowWindow(lbl, SW_HIDE);
            let _ = ShowWindow(btn, SW_HIDE);
        }
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

fn default_click_through_presets(current: &HotkeyBinding) -> Vec<(String, HotkeyBinding)> {
    let mut presets = vec![
        (
            "Ctrl+Shift+Q".to_string(),
            HotkeyBinding {
                control: true,
                shift: true,
                alt: false,
                win: false,
                virtual_key: b'Q' as i32,
            },
        ),
        (
            "Ctrl+Shift+T".to_string(),
            HotkeyBinding {
                control: true,
                shift: true,
                alt: false,
                win: false,
                virtual_key: b'T' as i32,
            },
        ),
        (
            "Ctrl+Alt+Q".to_string(),
            HotkeyBinding {
                control: true,
                shift: false,
                alt: true,
                win: false,
                virtual_key: b'Q' as i32,
            },
        ),
        (
            "Alt+Q".to_string(),
            HotkeyBinding {
                control: false,
                shift: false,
                alt: true,
                win: false,
                virtual_key: b'Q' as i32,
            },
        ),
        (
            "关闭（无热键）".to_string(),
            HotkeyBinding {
                virtual_key: 0,
                ..Default::default()
            },
        ),
    ];
    if !presets.iter().any(|(_, b)| same_hk(b, current)) {
        presets.insert(0, (current.to_display_string(), current.clone()));
    }
    presets
}

fn same_hk(a: &HotkeyBinding, b: &HotkeyBinding) -> bool {
    a.control == b.control
        && a.shift == b.shift
        && a.alt == b.alt
        && a.win == b.win
        && a.virtual_key == b.virtual_key
}

fn set_text(hwnd: HWND, text: &str) {
    if !hwnd.is_invalid() {
        let w = to_wide_null(text);
        unsafe {
            let _ = SetWindowTextW(hwnd, PCWSTR(w.as_ptr()));
        }
    }
}

fn get_text(hwnd: HWND) -> String {
    if hwnd.is_invalid() {
        return String::new();
    }
    let mut buf = vec![0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

fn set_check(hwnd: HWND, checked: bool) {
    if !hwnd.is_invalid() {
        unsafe {
            let _ = SendMessageW(
                hwnd,
                BM_SETCHECK,
                Some(WPARAM(if checked { BST_CHECKED } else { BST_UNCHECKED })),
                Some(LPARAM(0)),
            );
        }
    }
}

fn is_checked(hwnd: HWND) -> bool {
    !hwnd.is_invalid()
        && unsafe { SendMessageW(hwnd, BM_GETCHECK, None, None).0 == BST_CHECKED as isize }
}

fn build_settings_controls(hwnd: HWND, state: &Rc<RefCell<SettingsState>>) {
    let (
        dpi,
        dark,
        font_body,
        font_small,
        cap_keys,
        fn_keys,
        scroll_keys,
        ct_presets,
        hist_max_options,
    ) = {
        let st = state.borrow();
        (
            st.dpi,
            !st.light,
            st.font_body,
            st.font_small,
            st.cap_keys.clone(),
            st.fn_keys.clone(),
            st.scroll_keys.clone(),
            st.ct_presets.clone(),
            st.hist_max_options.clone(),
        )
    };
    let s = |dip: i32| s_px(dip, dpi);
    let btn_cls = to_wide_null("BUTTON");
    let lbl_cls = to_wide_null("STATIC");
    let edit_cls = to_wide_null("EDIT");
    let cmb_cls = to_wide_null("COMBOBOX");
    let trk_cls = to_wide_null("msctls_trackbar32");

    let themed: RefCell<Vec<HWND>> = RefCell::new(Vec::new());

    let mk_lbl_f = |text: &str, x: i32, y: i32, w: i32, h: i32, fnt: HFONT| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(lbl_cls.as_ptr()),
                PCWSTR(to_wide_null(text).as_ptr()),
                WS_CHILD | WS_VISIBLE,
                s(x),
                s(y),
                s(w),
                s(h),
                Some(hwnd),
                None,
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, fnt);
        hw
    };
    let mk_lbl = |text: &str, x: i32, y: i32, w: i32, h: i32| -> HWND {
        mk_lbl_f(text, x, y, w, h, font_body)
    };
    let mk_help = |text: &str, x: i32, y: i32, w: i32, h: i32| -> HWND {
        mk_lbl_f(text, x, y, w, h, font_small)
    };

    let mk_chk = |text: &str, x: i32, y: i32, w: i32, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(btn_cls.as_ptr()),
                PCWSTR(to_wide_null(text).as_ptr()),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                s(x),
                s(y),
                s(w),
                s(24),
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, font_body);
        apply_control_theme(hw, dark);
        themed.borrow_mut().push(hw);
        hw
    };

    let mk_btn = |text: &str, x: i32, y: i32, w: i32, h: i32, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(btn_cls.as_ptr()),
                PCWSTR(to_wide_null(text).as_ptr()),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                s(x),
                s(y),
                s(w),
                s(h),
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, font_body);
        hw
    };

    let mk_cmb = |x: i32, y: i32, w: i32, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(cmb_cls.as_ptr()),
                PCWSTR::null(),
                WS_CHILD
                    | WS_VISIBLE
                    | WS_TABSTOP
                    | WS_VSCROLL
                    | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
                s(x),
                s(y),
                s(w),
                s(200),
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, font_body);
        apply_control_theme(hw, dark);
        themed.borrow_mut().push(hw);
        hw
    };

    let mk_edit = |x: i32, y: i32, w: i32, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(edit_cls.as_ptr()),
                PCWSTR::null(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                s(x),
                s(y),
                s(w),
                s(26),
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, font_body);
        apply_control_theme(hw, dark);
        themed.borrow_mut().push(hw);
        hw
    };

    let mk_slider = |x: i32, y: i32, w: i32, min: isize, max: isize, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(trk_cls.as_ptr()),
                PCWSTR::null(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(TBS_AUTOTICKS),
                s(x),
                s(y),
                s(w),
                s(28),
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        unsafe {
            let lp = ((max as usize) << 16) | (min as usize & 0xFFFF);
            let _ = SendMessageW(hw, TBM_SETRANGE, Some(WPARAM(1)), Some(LPARAM(lp as isize)));
        }
        apply_control_theme(hw, dark);
        themed.borrow_mut().push(hw);
        hw
    };

    // Footer controls (always visible)
    let status_lbl = mk_help("就绪", 232, 524, 290, 22);
    let _b_res = mk_btn("恢复默认", 532, 519, 96, 30, ID_BTN_RESTORE);
    let _b_cls = mk_btn("关闭", 638, 519, 96, 30, ID_BTN_CLOSE);

    let px = 232;

    // ------------------------------------------------------------------
    // Page 0: General
    // ------------------------------------------------------------------
    let mut p0 = Vec::new();
    let chk_run = mk_chk("开机启动（写入当前用户 Run 注册表）", px, 86, 360, ID_CHK_RUN_STARTUP);
    let b_theme = mk_btn("切换系统深浅色 (Apps + System)", px, 118, 256, 30, ID_BTN_TOGGLE_THEME);
    let b_qc = mk_btn("区域截图", px, 198, 78, 28, ID_BTN_QUICK_CAPTURE);
    let b_qdelay = mk_btn("延时3秒", px + 84, 198, 76, 28, ID_BTN_QUICK_DELAYED);
    let b_qp = mk_btn("取色", px + 166, 198, 56, 28, ID_BTN_QUICK_PICK);
    let b_qs = mk_btn("滚动截图", px + 228, 198, 78, 28, ID_BTN_QUICK_SCROLL);
    let b_qpins = mk_btn("贴图管理", px + 312, 198, 78, 28, ID_BTN_QUICK_PINS);
    let b_qhist = mk_btn("截图历史", px + 396, 198, 78, 28, ID_BTN_OPEN_HISTORY);
    let lbl_sum = mk_lbl("", px, 278, 504, 72);
    let lbl_g_help = mk_help("所有设置即时应用并自动保存至 settings.json。", px, 362, 480, 22);
    p0.extend([
        chk_run, b_theme, b_qc, b_qdelay, b_qp, b_qs, b_qpins, b_qhist, lbl_sum, lbl_g_help,
    ]);

    // ------------------------------------------------------------------
    // Page 1: Capture & Hotkeys
    // ------------------------------------------------------------------
    let mut p1 = Vec::new();
    p1.push(mk_lbl("区域截图热键：", px, 62, 108, 22));
    let chk_ctrl = mk_chk("Ctrl", px + 108, 60, 56, ID_CHK_CTRL);
    let chk_shift = mk_chk("Shift", px + 168, 60, 62, ID_CHK_SHIFT);
    let chk_alt = mk_chk("Alt", px + 234, 60, 52, ID_CHK_ALT);
    let chk_win = mk_chk("Win", px + 290, 60, 54, ID_CHK_WIN);
    let cmb_cap = mk_cmb(px + 350, 58, 90, ID_CMB_CAP_KEY);
    for (_, label) in &cap_keys {
        let w = to_wide_null(label);
        unsafe {
            let _ = SendMessageW(cmb_cap, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    p1.extend([chk_ctrl, chk_shift, chk_alt, chk_win, cmb_cap]);

    let lbl_conflict = mk_help("", px, 90, 360, 20);
    let btn_try_next = mk_btn(
        HotkeyCopy::TRY_NEXT_BUTTON,
        px + 368,
        87,
        136,
        24,
        ID_BTN_TRY_NEXT_KEY,
    );
    unsafe {
        let _ = ShowWindow(lbl_conflict, SW_HIDE);
        let _ = ShowWindow(btn_try_next, SW_HIDE);
    }
    p1.extend([lbl_conflict, btn_try_next]);

    p1.push(mk_lbl("取色热键：", px, 120, 104, 22));
    let cmb_pick = mk_cmb(px + 108, 117, 94, ID_CMB_PICK_KEY);
    for (_, label) in &fn_keys {
        let w = to_wide_null(label);
        unsafe {
            let _ = SendMessageW(cmb_pick, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    p1.push(cmb_pick);

    p1.push(mk_lbl("滚动热键：", px + 220, 120, 74, 22));
    let cmb_scroll = mk_cmb(px + 296, 117, 118, ID_CMB_SCROLL_KEY);
    for (_, label) in &scroll_keys {
        let w = to_wide_null(label);
        unsafe {
            let _ = SendMessageW(cmb_scroll, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    let btn_scroll_now = mk_btn("立即滚动", px + 422, 116, 82, 26, ID_BTN_START_SCROLL);
    p1.extend([cmb_scroll, btn_scroll_now]);

    p1.push(mk_lbl("贴图穿透热键：", px, 154, 108, 22));
    let cmb_ct = mk_cmb(px + 108, 151, 152, ID_CMB_CLICK_THROUGH);
    for (label, _) in &ct_presets {
        let w = to_wide_null(label);
        unsafe {
            let _ = SendMessageW(cmb_ct, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    p1.push(cmb_ct);
    p1.push(mk_help("切换最上层贴图的鼠标穿透开关", px + 272, 155, 220, 20));

    let chk_mag = mk_chk("选区时显示放大镜", px, 224, 210, ID_CHK_MAGNIFIER);
    let chk_pin_after = mk_chk("截图后直接钉图（跳过工具栏）", px + 228, 224, 260, ID_CHK_PIN_AFTER);
    let chk_save = mk_chk("截图后自动保存到文件", px, 254, 210, ID_CHK_SAVE_FILE);
    let chk_hist = mk_chk("保留截图历史", px + 228, 254, 130, ID_CHK_HISTORY);
    let lbl_hmax = mk_lbl("上限", px + 366, 256, 36, 22);
    let cmb_hmax = mk_cmb(px + 404, 253, 72, ID_CMB_HIST_MAX);
    for &n in &hist_max_options {
        let w = to_wide_null(&format!("{n}张"));
        unsafe {
            let _ = SendMessageW(cmb_hmax, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    p1.extend([chk_mag, chk_pin_after, chk_save, chk_hist, lbl_hmax, cmb_hmax]);

    p1.push(mk_lbl("保存目录：", px, 290, 74, 22));
    let edit_dir = mk_edit(px + 76, 287, 236, ID_EDIT_SAVE_DIR);
    let btn_browse = mk_btn("浏览…", px + 320, 286, 56, 27, ID_BTN_BROWSE_DIR);
    let btn_open_dir = mk_btn("打开", px + 382, 286, 56, 27, ID_BTN_OPEN_DIR);
    let btn_hist = mk_btn("历史", px + 444, 286, 60, 27, ID_BTN_OPEN_HISTORY);
    p1.extend([edit_dir, btn_browse, btn_open_dir, btn_hist]);
    p1.push(mk_help(
        &format!("留空则默认保存至：{}", SettingsPaths::default_capture_directory().display()),
        px,
        322,
        504,
        36,
    ));

    // ------------------------------------------------------------------
    // Page 2: NetSpeed
    // ------------------------------------------------------------------
    let mut p2 = Vec::new();
    let chk_nv = mk_chk("显示网速", px, 60, 102, ID_CHK_NET_VISIBLE);
    let chk_ne = mk_chk("嵌入主任务栏", px + 114, 60, 138, ID_CHK_NET_EMBED);
    let chk_ns = mk_chk("同时嵌入副屏任务栏", px + 266, 60, 186, ID_CHK_NET_SEC);
    p2.extend([chk_nv, chk_ne, chk_ns]);

    p2.push(mk_lbl("监听网卡：", px, 94, 72, 22));
    let cmb_ad = mk_cmb(px + 76, 91, 344, ID_CMB_ADAPTER);
    let btn_ref_ad = mk_btn("刷新", px + 428, 90, 76, 27, ID_BTN_REFRESH_ADAPTER);
    p2.extend([cmb_ad, btn_ref_ad]);

    p2.push(mk_lbl("字体大小：", px, 168, 72, 22));
    let sld_font = mk_slider(px + 76, 164, 220, 10, 18, ID_SLD_FONT);
    let lbl_font_val = mk_lbl("13 px", px + 306, 168, 60, 22);
    p2.extend([sld_font, lbl_font_val]);

    let chk_sd = mk_chk("显示下行", px, 198, 105, ID_CHK_SHOW_DOWN);
    let chk_su = mk_chk("显示上行", px + 114, 198, 105, ID_CHK_SHOW_UP);
    let chk_tl = mk_chk("双行显示", px + 228, 198, 105, ID_CHK_TWO_LINE);
    let chk_bd = mk_chk("粗体字重", px + 342, 198, 100, ID_CHK_BOLD);
    p2.extend([chk_sd, chk_su, chk_tl, chk_bd]);

    let chk_bgt = mk_chk("背景透明", px, 230, 105, ID_CHK_BG_TRANS);
    let btn_bgc = mk_btn("背景色", px + 114, 227, 108, 28, ID_BTN_BG_COLOR);
    let chk_bdr = mk_chk("显示边框", px + 236, 230, 110, ID_CHK_BORDER);
    p2.extend([chk_bgt, btn_bgc, chk_bdr]);

    let btn_dc = mk_btn("下行颜色", px, 263, 114, 28, ID_BTN_DOWN_COLOR);
    let btn_dr = mk_btn("重置", px + 120, 263, 58, 28, ID_BTN_DOWN_RESET);
    let btn_uc = mk_btn("上行颜色", px + 196, 263, 114, 28, ID_BTN_UP_COLOR);
    let btn_ur = mk_btn("重置", px + 316, 263, 58, 28, ID_BTN_UP_RESET);
    p2.extend([btn_dc, btn_dr, btn_uc, btn_ur]);
    p2.push(mk_help(NetSpeedCopy::EMBED_HELP, px, 414, 504, 44));

    // ------------------------------------------------------------------
    // Page 3: TaskbarFx
    // ------------------------------------------------------------------
    let mut p3 = Vec::new();
    let chk_fx_on = mk_chk("启用任务栏透明与背景外观", px, 60, 240, ID_CHK_FX_ENABLED);
    p3.push(chk_fx_on);

    p3.push(mk_lbl("外观模式：", px, 96, 86, 22));
    let cmb_fx_m = mk_cmb(px + 90, 93, 150, ID_CMB_FX_MODE);
    for wire in [
        TaskbarAppearanceModes::NORMAL,
        TaskbarAppearanceModes::OPAQUE,
        TaskbarAppearanceModes::CLEAR,
        TaskbarAppearanceModes::ACRYLIC,
    ] {
        let w = to_wide_null(TaskbarFxCopy::mode_label(Some(wire)));
        unsafe {
            let _ = SendMessageW(cmb_fx_m, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    let lbl_fx_m_help = mk_help("", px + 250, 97, 254, 22);
    p3.extend([cmb_fx_m, lbl_fx_m_help]);

    p3.push(mk_lbl("不透明度：", px, 134, 86, 22));
    let sld_alpha = mk_slider(px + 90, 130, 220, 0, 255, ID_SLD_FX_ALPHA);
    let lbl_alpha = mk_lbl("204", px + 320, 134, 48, 22);
    p3.extend([sld_alpha, lbl_alpha]);

    p3.push(mk_lbl("ARGB 颜色：", px, 170, 86, 22));
    let edit_argb = mk_edit(px + 90, 167, 130, ID_EDIT_FX_ARGB);
    let btn_fx_col = mk_btn("颜色选择", px + 230, 166, 114, 28, ID_BTN_FX_COLOR);
    p3.extend([edit_argb, btn_fx_col]);

    let chk_fx_keep = mk_chk("Suite 退出后保持任务栏外观", px, 206, 280, ID_CHK_FX_KEEP);
    let lbl_fx_status = mk_help("", px, 282, 504, 28);
    let lbl_light_tip = mk_help(
        "浅色主题下，全透明任务栏上的网速文字可能不清晰；建议关掉网速背景透明或改用亚克力方案。",
        px,
        318,
        504,
        36,
    );
    let btn_use_acr = mk_btn("改用亚克力可读方案", px, 362, 164, 30, ID_BTN_USE_ACRYLIC);
    p3.extend([
        chk_fx_keep,
        lbl_fx_status,
        lbl_light_tip,
        btn_use_acr,
    ]);

    // ------------------------------------------------------------------
    // Page 4: Advanced & About
    // ------------------------------------------------------------------
    let mut p4 = Vec::new();
    let chk_log = mk_chk("记录调试日志", px, 86, 160, ID_CHK_LOGGING);
    let btn_logs = mk_btn("打开日志文件夹", px + 356, 84, 148, 28, ID_BTN_OPEN_LOGS);
    p4.extend([chk_log, btn_logs]);
    p4.push(mk_help(
        &format!("日志目录：{}", SettingsPaths::logs_directory().display()),
        px,
        118,
        504,
        22,
    ));

    let chk_autooff = mk_chk(
        "任务栏透明异常退出后自动关闭效果（保住截图与网速）",
        px,
        148,
        480,
        ID_CHK_FX_AUTOOFF,
    );
    let lbl_last = mk_help("上次启动：正常", px, 178, 280, 22);
    p4.extend([chk_autooff, lbl_last]);

    p4.push(mk_lbl("更新通道：", px, 254, 84, 22));
    let cmb_chan = mk_cmb(px + 90, 251, 120, ID_CMB_CHANNEL);
    for ch in ["正式", "预览"] {
        let w = to_wide_null(ch);
        unsafe {
            let _ = SendMessageW(cmb_chan, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    let btn_upd = mk_btn("检查更新", px + 222, 250, 108, 28, ID_BTN_CHECK_UPDATE);
    let btn_open_rel = mk_btn("打开下载页 →", px + 340, 250, 126, 28, ID_BTN_OPEN_RELEASE);
    unsafe {
        let _ = ShowWindow(btn_open_rel, SW_HIDE);
    }
    p4.extend([cmb_chan, btn_upd, btn_open_rel]);

    let lbl_ver = mk_lbl(
        &format!("当前版本：{} (Rust Native · Win32/D2D)", UpdateChecker::LOCAL_VERSION),
        px,
        294,
        340,
        22,
    );
    let btn_gh = mk_btn("GitHub 仓库", px + 356, 290, 148, 28, ID_BTN_GITHUB);
    let lbl_lic = mk_help(
        "许可协议：GPL-3.0 · TaskbarFx.Tap 遵循 TranslucentTB 开源协议归属",
        px,
        328,
        504,
        22,
    );
    p4.extend([lbl_ver, btn_gh, lbl_lic]);

    let mut st = state.borrow_mut();
    st.themed_controls = themed.into_inner();
    st.status_lbl = status_lbl;
    st.fx_status_lbl = lbl_fx_status;
    st.hotkey_conflict_lbl = lbl_conflict;
    st.try_next_btn = btn_try_next;
    st.light_tip_lbl = lbl_light_tip;
    st.use_acrylic_btn = btn_use_acr;
    st.btn_open_release = btn_open_rel;
    st.chk_run_startup = chk_run;
    st.lbl_general_summary = lbl_sum;
    st.chk_ctrl = chk_ctrl;
    st.chk_shift = chk_shift;
    st.chk_alt = chk_alt;
    st.chk_win = chk_win;
    st.cmb_cap_key = cmb_cap;
    st.cmb_pick_key = cmb_pick;
    st.cmb_scroll_key = cmb_scroll;
    st.cmb_click_through = cmb_ct;
    st.chk_magnifier = chk_mag;
    st.chk_save_file = chk_save;
    st.edit_save_dir = edit_dir;
    st.chk_pin_after = chk_pin_after;
    st.chk_history = chk_hist;
    st.cmb_hist_max = cmb_hmax;
    st.chk_net_vis = chk_nv;
    st.chk_net_embed = chk_ne;
    st.chk_net_sec = chk_ns;
    st.cmb_adapter = cmb_ad;
    st.sld_font = sld_font;
    st.lbl_font_val = lbl_font_val;
    st.chk_show_down = chk_sd;
    st.chk_show_up = chk_su;
    st.chk_two_line = chk_tl;
    st.chk_bold = chk_bd;
    st.chk_bg_trans = chk_bgt;
    st.chk_border = chk_bdr;
    st.btn_bg_color = btn_bgc;
    st.btn_down_color = btn_dc;
    st.btn_up_color = btn_uc;
    st.chk_fx_enabled = chk_fx_on;
    st.cmb_fx_mode = cmb_fx_m;
    st.lbl_fx_mode_help = lbl_fx_m_help;
    st.sld_fx_alpha = sld_alpha;
    st.lbl_fx_alpha = lbl_alpha;
    st.edit_fx_argb = edit_argb;
    st.btn_fx_color = btn_fx_col;
    st.chk_fx_keep = chk_fx_keep;
    st.chk_logging = chk_log;
    st.chk_fx_autooff = chk_autooff;
    st.lbl_last_start = lbl_last;
    st.cmb_channel = cmb_chan;
    st.pages = [p0, p1, p2, p3, p4];
}

fn select_nav(state: &Rc<RefCell<SettingsState>>, index: usize) {
    let (hwnd, pages, conflict_nonempty, dl_url) = {
        let mut st = state.borrow_mut();
        st.current_nav = index;
        (
            st.hwnd,
            st.pages.clone(),
            !get_text(st.hotkey_conflict_lbl).trim().is_empty(),
            st.download_url.clone(),
        )
    };
    for (i, group) in pages.iter().enumerate() {
        let cmd = if i == index { SW_SHOW } else { SW_HIDE };
        for &h in group {
            unsafe {
                let _ = ShowWindow(h, cmd);
            }
        }
    }
    if index == 1 {
        let st = state.borrow();
        let cmd = if conflict_nonempty { SW_SHOW } else { SW_HIDE };
        unsafe {
            let _ = ShowWindow(st.hotkey_conflict_lbl, cmd);
            let _ = ShowWindow(st.try_next_btn, cmd);
        }
    } else if index == 3 {
        refresh_light_tip(state);
    } else if index == 4 {
        let st = state.borrow();
        let cmd = if dl_url.is_some() { SW_SHOW } else { SW_HIDE };
        unsafe {
            let _ = ShowWindow(st.btn_open_release, cmd);
        }
    }
    invalidate_all(hwnd);
}

fn populate_controls_from_state(state: &Rc<RefCell<SettingsState>>, last_start_label: &str) {
    let st = state.borrow();
    let s = &st.settings_snapshot;

    set_check(st.chk_run_startup, s.start_with_windows);
    let ws_mb = current_working_set_mb();
    let summary = format!(
        "当前版本：v{} (Rust Native) · 识别显示器：{} 台 · 内存 Working Set：{:.1} MB\n截图热键：{} · 取色：{} · 滚动：{} · 贴图穿透：{}\n任务栏网速：{} ({}) · 任务栏透明：{}",
        UpdateChecker::LOCAL_VERSION,
        st.monitor_count.max(1),
        ws_mb,
        if s.hotkey.is_disabled() {
            "关".to_string()
        } else {
            s.hotkey.to_display_string()
        },
        s.capture.color_pick_hotkey.to_display_string(),
        if s.capture.scroll_capture_hotkey.is_disabled() {
            "关".to_string()
        } else {
            s.capture.scroll_capture_hotkey.to_display_string()
        },
        if s.capture.pin_click_through_hotkey.is_disabled() {
            "关".to_string()
        } else {
            s.capture.pin_click_through_hotkey.to_display_string()
        },
        if s.net_speed.visible { "开" } else { "关" },
        s.net_speed
            .adapter_alias
            .as_deref()
            .filter(|a| !a.trim().is_empty())
            .unwrap_or("自动选择"),
        if s.taskbar_fx.enabled {
            TaskbarFxCopy::mode_label(Some(&s.taskbar_fx.mode))
        } else {
            "关"
        }
    );
    set_text(st.lbl_general_summary, &summary);

    set_check(st.chk_ctrl, s.hotkey.control);
    set_check(st.chk_shift, s.hotkey.shift);
    set_check(st.chk_alt, s.hotkey.alt);
    set_check(st.chk_win, s.hotkey.win);
    let cap_idx = st
        .cap_keys
        .iter()
        .position(|(vk, _)| *vk == s.hotkey.virtual_key)
        .unwrap_or(0);
    unsafe {
        let _ = SendMessageW(st.cmb_cap_key, CB_SETCURSEL, Some(WPARAM(cap_idx)), Some(LPARAM(0)));
    }
    let pick_idx = st
        .fn_keys
        .iter()
        .position(|(vk, _)| *vk == s.capture.color_pick_hotkey.virtual_key)
        .unwrap_or(1);
    unsafe {
        let _ = SendMessageW(st.cmb_pick_key, CB_SETCURSEL, Some(WPARAM(pick_idx)), Some(LPARAM(0)));
    }
    let scr_idx = st
        .scroll_keys
        .iter()
        .position(|(vk, _)| *vk == s.capture.scroll_capture_hotkey.virtual_key)
        .unwrap_or(4);
    unsafe {
        let _ = SendMessageW(st.cmb_scroll_key, CB_SETCURSEL, Some(WPARAM(scr_idx)), Some(LPARAM(0)));
    }
    let ct_idx = st
        .ct_presets
        .iter()
        .position(|(_, b)| same_hk(b, &s.capture.pin_click_through_hotkey))
        .unwrap_or(0);
    unsafe {
        let _ = SendMessageW(st.cmb_click_through, CB_SETCURSEL, Some(WPARAM(ct_idx)), Some(LPARAM(0)));
    }
    set_check(st.chk_magnifier, s.capture.show_magnifier);
    set_check(st.chk_save_file, s.capture.save_file_after_capture);
    set_text(st.edit_save_dir, &s.capture.save_directory);
    set_check(st.chk_pin_after, s.capture.pin_after_capture);
    set_check(st.chk_history, s.capture.history_enabled);
    let hmax_idx = st
        .hist_max_options
        .iter()
        .position(|&v| v == s.capture.history_max)
        .unwrap_or(2);
    unsafe {
        let _ = SendMessageW(st.cmb_hist_max, CB_SETCURSEL, Some(WPARAM(hmax_idx)), Some(LPARAM(0)));
    }

    set_check(st.chk_net_vis, s.net_speed.visible);
    set_check(st.chk_net_embed, s.net_speed.embed_in_taskbar);
    set_check(st.chk_net_sec, s.net_speed.embed_secondary);
    unsafe {
        let _ = SendMessageW(st.cmb_adapter, CB_RESETCONTENT, None, None);
        let auto_label = to_wide_null("自动选择（优先活跃物理网卡）");
        let _ = SendMessageW(
            st.cmb_adapter,
            CB_ADDSTRING,
            None,
            Some(LPARAM(auto_label.as_ptr() as isize)),
        );
        for a in &st.adapters {
            let w = to_wide_null(&a.display);
            let _ = SendMessageW(st.cmb_adapter, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
        let ad_idx = match s.net_speed.if_index {
            None => 0,
            Some(idx) => st
                .adapters
                .iter()
                .position(|a| a.if_index == idx)
                .map(|p| p + 1)
                .unwrap_or(0),
        };
        let _ = SendMessageW(st.cmb_adapter, CB_SETCURSEL, Some(WPARAM(ad_idx)), Some(LPARAM(0)));
    }
    let fs = s.net_speed.font_size_dip.round().clamp(10.0, 18.0) as isize;
    unsafe {
        let _ = SendMessageW(st.sld_font, TBM_SETPOS, Some(WPARAM(1)), Some(LPARAM(fs)));
    }
    set_text(st.lbl_font_val, &format!("{fs} px"));
    set_check(st.chk_show_down, s.net_speed.show_download);
    set_check(st.chk_show_up, s.net_speed.show_upload);
    set_check(st.chk_two_line, s.net_speed.two_line);
    set_check(st.chk_bold, s.net_speed.bold);
    set_check(st.chk_bg_trans, s.net_speed.background_transparent);
    set_check(st.chk_border, s.net_speed.show_border);

    set_check(st.chk_fx_enabled, s.taskbar_fx.enabled);
    let mode_idx = match s.taskbar_fx.mode.to_ascii_lowercase().as_str() {
        "normal" => 0,
        "opaque" => 1,
        "clear" => 2,
        _ => 3,
    };
    unsafe {
        let _ = SendMessageW(st.cmb_fx_mode, CB_SETCURSEL, Some(WPARAM(mode_idx)), Some(LPARAM(0)));
    }
    set_text(st.lbl_fx_mode_help, TaskbarFxCopy::mode_help(Some(&s.taskbar_fx.mode)));
    let alpha = ((s.taskbar_fx.argb >> 24) & 0xFF) as isize;
    unsafe {
        let _ = SendMessageW(st.sld_fx_alpha, TBM_SETPOS, Some(WPARAM(1)), Some(LPARAM(alpha)));
    }
    set_text(st.lbl_fx_alpha, &alpha.to_string());
    set_text(st.edit_fx_argb, &format!("#{:08X}", s.taskbar_fx.argb));
    set_check(st.chk_fx_keep, s.taskbar_fx.keep_appearance_if_suite_exits);

    set_check(st.chk_logging, s.advanced.logging_enabled);
    set_check(st.chk_fx_autooff, s.advanced.taskbar_fx_auto_disable_on_fail);
    set_text(st.lbl_last_start, &format!("上次启动：{last_start_label}"));
    let ch_idx = if s.update.channel.eq_ignore_ascii_case(UpdateSettings::CHANNEL_PREVIEW) {
        1
    } else {
        0
    };
    unsafe {
        let _ = SendMessageW(st.cmb_channel, CB_SETCURSEL, Some(WPARAM(ch_idx)), Some(LPARAM(0)));
    }
}

fn refresh_light_tip(state: &Rc<RefCell<SettingsState>>) {
    let st = state.borrow();
    if st.current_nav != 3 {
        return;
    }
    let light = PersonalizeRegistry::try_read()
        .map(|p| p.apps_use_light)
        .unwrap_or(true);
    let fx_on = is_checked(st.chk_fx_enabled);
    let mode_idx = unsafe { SendMessageW(st.cmb_fx_mode, CB_GETCURSEL, None, None).0 };
    let clear_or_acrylic = mode_idx == 2 || mode_idx == 3;
    let net_trans = is_checked(st.chk_bg_trans);
    let show = light && ((fx_on && clear_or_acrylic) || net_trans);
    let cmd = if show { SW_SHOW } else { SW_HIDE };
    unsafe {
        let _ = ShowWindow(st.light_tip_lbl, cmd);
        let _ = ShowWindow(st.use_acrylic_btn, cmd);
    }
}

fn read_settings_from_ui(state: &Rc<RefCell<SettingsState>>) -> AppSettings {
    let st = state.borrow();
    let mut next = st.settings_snapshot.clone();

    next.start_with_windows = is_checked(st.chk_run_startup);

    next.hotkey.control = is_checked(st.chk_ctrl);
    next.hotkey.shift = is_checked(st.chk_shift);
    next.hotkey.alt = is_checked(st.chk_alt);
    next.hotkey.win = is_checked(st.chk_win);
    let cap_sel = unsafe { SendMessageW(st.cmb_cap_key, CB_GETCURSEL, None, None).0 };
    if let Some(&(vk, _)) = st.cap_keys.get(cap_sel.max(0) as usize) {
        next.hotkey.virtual_key = vk;
    }
    let pick_sel = unsafe { SendMessageW(st.cmb_pick_key, CB_GETCURSEL, None, None).0 };
    if let Some(&(vk, _)) = st.fn_keys.get(pick_sel.max(0) as usize) {
        next.capture.color_pick_hotkey = HotkeyBinding {
            virtual_key: vk,
            ..Default::default()
        };
    }
    let scr_sel = unsafe { SendMessageW(st.cmb_scroll_key, CB_GETCURSEL, None, None).0 };
    if let Some(&(vk, _)) = st.scroll_keys.get(scr_sel.max(0) as usize) {
        next.capture.scroll_capture_hotkey = HotkeyBinding {
            virtual_key: vk,
            ..Default::default()
        };
    }
    let ct_sel = unsafe { SendMessageW(st.cmb_click_through, CB_GETCURSEL, None, None).0 };
    if let Some((_, bind)) = st.ct_presets.get(ct_sel.max(0) as usize) {
        next.capture.pin_click_through_hotkey = bind.clone();
    }
    next.capture.show_magnifier = is_checked(st.chk_magnifier);
    next.capture.save_file_after_capture = is_checked(st.chk_save_file);
    next.capture.save_directory = get_text(st.edit_save_dir).trim().to_string();
    next.capture.pin_after_capture = is_checked(st.chk_pin_after);
    next.capture.history_enabled = is_checked(st.chk_history);
    let hmax_sel = unsafe { SendMessageW(st.cmb_hist_max, CB_GETCURSEL, None, None).0 };
    if let Some(&n) = st.hist_max_options.get(hmax_sel.max(0) as usize) {
        next.capture.history_max = n;
    }

    next.net_speed.visible = is_checked(st.chk_net_vis);
    next.net_speed.embed_in_taskbar = is_checked(st.chk_net_embed);
    next.net_speed.embed_secondary = is_checked(st.chk_net_sec);
    let ad_sel = unsafe { SendMessageW(st.cmb_adapter, CB_GETCURSEL, None, None).0 };
    if ad_sel <= 0 {
        next.net_speed.if_index = None;
        next.net_speed.adapter_alias = None;
    } else if let Some(a) = st.adapters.get((ad_sel - 1) as usize) {
        next.net_speed.if_index = Some(a.if_index);
        next.net_speed.adapter_alias = Some(a.alias.clone());
    }
    let font_pos = unsafe { SendMessageW(st.sld_font, TBM_GETPOS, None, None).0 };
    next.net_speed.font_size_dip = (font_pos as f64).clamp(10.0, 18.0);
    next.net_speed.show_download = is_checked(st.chk_show_down);
    next.net_speed.show_upload = is_checked(st.chk_show_up);
    next.net_speed.two_line = is_checked(st.chk_two_line);
    next.net_speed.bold = is_checked(st.chk_bold);
    next.net_speed.background_transparent = is_checked(st.chk_bg_trans);
    next.net_speed.background_argb = st.bg_argb;
    next.net_speed.down_color_argb = st.down_argb;
    next.net_speed.up_color_argb = st.up_argb;
    next.net_speed.show_border = is_checked(st.chk_border);
    next.net_speed.normalize();

    next.taskbar_fx.enabled = is_checked(st.chk_fx_enabled);
    let mode_idx = unsafe { SendMessageW(st.cmb_fx_mode, CB_GETCURSEL, None, None).0 };
    next.taskbar_fx.mode = match mode_idx {
        0 => TaskbarAppearanceModes::NORMAL,
        1 => TaskbarAppearanceModes::OPAQUE,
        2 => TaskbarAppearanceModes::CLEAR,
        _ => TaskbarAppearanceModes::ACRYLIC,
    }
    .to_string();
    let raw_argb = get_text(st.edit_fx_argb);
    next.taskbar_fx.argb = parse_argb_hex(&raw_argb, next.taskbar_fx.argb);
    next.taskbar_fx.keep_appearance_if_suite_exits = is_checked(st.chk_fx_keep);

    next.advanced.logging_enabled = is_checked(st.chk_logging);
    next.advanced.taskbar_fx_auto_disable_on_fail = is_checked(st.chk_fx_autooff);
    let ch_idx = unsafe { SendMessageW(st.cmb_channel, CB_GETCURSEL, None, None).0 };
    next.update.channel = if ch_idx == 1 {
        UpdateSettings::CHANNEL_PREVIEW.to_string()
    } else {
        UpdateSettings::CHANNEL_STABLE.to_string()
    };

    next
}

fn apply_now(state: &Rc<RefCell<SettingsState>>) {
    if state.borrow().loading {
        return;
    }
    let next = read_settings_from_ui(state);
    let (hwnd, controller, status_lbl, nav) = {
        let mut st = state.borrow_mut();
        st.settings_snapshot = next.clone();
        (st.hwnd, st.controller, st.status_lbl, st.current_nav)
    };
    set_text(status_lbl, "已自动保存");
    post_boxed(controller, WM_APP_SETTINGS_APPLY, 0, next);
    if nav == 2 || nav == 3 {
        invalidate_all(hwnd);
    }
}

fn parse_argb_hex(text: &str, fallback: u32) -> u32 {
    let trimmed = text.trim();
    let hex = trimmed
        .strip_prefix('#')
        .or_else(|| trimmed.strip_prefix("0x"))
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if hex.len() == 6 && let Ok(rgb) = u32::from_str_radix(hex, 16) {
        return 0xFF00_0000 | rgb;
    }
    if hex.len() == 8 && let Ok(argb) = u32::from_str_radix(hex, 16) {
        return argb;
    }
    fallback
}

fn argb_to_colorref_local(argb: u32) -> COLORREF {
    let r = (argb >> 16) & 0xFF;
    let g = (argb >> 8) & 0xFF;
    let b = argb & 0xFF;
    COLORREF((b << 16) | (g << 8) | r)
}

fn nav_item_rect_px(index: usize, dpi: u32) -> RECT {
    let s = |dip: i32| s_px(dip, dpi);
    let top = 44 + (index as i32) * 40 + 4;
    RECT {
        left: s(12),
        top: s(top),
        right: s(NAV_W_DIP - 12),
        bottom: s(top + 32),
    }
}

fn hit_test_nav(x: i32, y: i32, dpi: u32) -> Option<usize> {
    if x < 0 || x >= s_px(NAV_W_DIP, dpi) {
        return None;
    }
    for (i, _) in NAV_LABELS.iter().enumerate() {
        let rc = nav_item_rect_px(i, dpi);
        if x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom {
            return Some(i);
        }
    }
    None
}

fn draw_hairline(hdc: HDC, x1: i32, y1: i32, x2: i32, y2: i32, col: COLORREF) {
    unsafe {
        let pen = CreatePen(PS_SOLID, 1, col);
        let old = SelectObject(hdc, pen.into());
        let _ = MoveToEx(hdc, x1, y1, None);
        let _ = LineTo(hdc, x2, y2);
        SelectObject(hdc, old);
        let _ = DeleteObject(pen.into());
    }
}

fn paint_settings_chrome(hwnd: HWND, state: &Rc<RefCell<SettingsState>>) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    if hdc.is_invalid() {
        return;
    }
    let cr = client_rect(hwnd);
    let cw = (cr.right - cr.left).max(1);
    let ch = (cr.bottom - cr.top).max(1);

    let (
        dpi,
        light,
        cur_nav,
        hover_nav,
        f_nav_hdr,
        f_body,
        f_section,
        f_title,
        d2d,
        ns,
        theme_kind,
    ) = {
        let st = state.borrow();
        (
            st.dpi,
            st.light,
            st.current_nav,
            st.hover_nav,
            st.font_nav_hdr,
            st.font_body,
            st.font_section,
            st.font_title,
            Rc::clone(&st.d2d),
            st.settings_snapshot.net_speed.clone(),
            if st.light { ThemeKind::Light } else { ThemeKind::Dark },
        )
    };
    let s = |dip: i32| s_px(dip, dpi);
    let pal = palette(light);

    unsafe {
        let mem_dc = CreateCompatibleDC(Some(hdc));
        let mem_bmp = CreateCompatibleBitmap(hdc, cw, ch);
        let old_bmp = SelectObject(mem_dc, mem_bmp.into());

        let nav_w = s(NAV_W_DIP);
        let footer_top = ch - s(FOOTER_H_DIP);

        // Backgrounds
        draw_filled_rect(
            mem_dc,
            RECT {
                left: 0,
                top: 0,
                right: nav_w,
                bottom: ch,
            },
            pal.nav_bg,
        );
        draw_filled_rect(
            mem_dc,
            RECT {
                left: nav_w,
                top: 0,
                right: cw,
                bottom: footer_top,
            },
            pal.body_bg,
        );
        draw_filled_rect(
            mem_dc,
            RECT {
                left: nav_w,
                top: footer_top,
                right: cw,
                bottom: ch,
            },
            pal.footer_bg,
        );

        // Vertical nav divider & footer top hairline
        draw_hairline(mem_dc, nav_w - 1, 0, nav_w - 1, ch, pal.line);
        draw_hairline(mem_dc, nav_w, footer_top, cw, footer_top, pal.line);

        // Nav rail header & items
        SetBkMode(mem_dc, TRANSPARENT);
        let old_f = SelectObject(mem_dc, f_nav_hdr.into());
        SetTextColor(mem_dc, pal.ink_muted);
        let mut hdr_rc = RECT {
            left: s(20),
            top: s(14),
            right: nav_w - s(16),
            bottom: s(34),
        };
        let mut hdr_w: Vec<u16> = "设置".encode_utf16().collect();
        DrawTextW(mem_dc, &mut hdr_w, &mut hdr_rc, DT_LEFT | DT_VCENTER | DT_SINGLELINE);

        for (i, label) in NAV_LABELS.iter().enumerate() {
            let rc = nav_item_rect_px(i, dpi);
            let active = i == cur_nav;
            let hovered = !active && hover_nav == Some(i);
            if active {
                draw_rounded_rect(
                    mem_dc,
                    rc,
                    s(6),
                    pal.pill_active,
                    light.then_some(pal.line),
                );
                let pill_mid = (rc.top + rc.bottom) / 2;
                let bar_h = s(18);
                draw_rounded_rect(
                    mem_dc,
                    RECT {
                        left: rc.left + s(6),
                        top: pill_mid - bar_h / 2,
                        right: rc.left + s(9),
                        bottom: pill_mid + bar_h / 2,
                    },
                    s(2),
                    pal.accent,
                    None,
                );
                SelectObject(mem_dc, f_section.into());
                SetTextColor(mem_dc, pal.ink);
            } else {
                if hovered {
                    draw_rounded_rect(mem_dc, rc, s(6), pal.pill_hover, None);
                }
                SelectObject(mem_dc, f_body.into());
                SetTextColor(mem_dc, pal.ink_secondary);
            }
            let mut trc = RECT {
                left: rc.left + s(18),
                top: rc.top,
                right: rc.right - s(10),
                bottom: rc.bottom,
            };
            let mut wt: Vec<u16> = label.encode_utf16().collect();
            DrawTextW(mem_dc, &mut wt, &mut trc, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }

        // Right-pane Page Header + hairline
        let content_x = s(232);
        let content_r = s(736);
        SelectObject(mem_dc, f_title.into());
        SetTextColor(mem_dc, pal.ink);
        let mut title_rc = RECT {
            left: content_x,
            top: s(12),
            right: content_r,
            bottom: s(44),
        };
        let mut title_w: Vec<u16> = NAV_LABELS[cur_nav].encode_utf16().collect();
        DrawTextW(mem_dc, &mut title_w, &mut title_rc, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        draw_hairline(mem_dc, content_x, s(48), content_r, s(48), pal.line);

        let draw_sec = |hdc_in: HDC, label: &str, y_dip: i32| {
            SelectObject(hdc_in, f_section.into());
            SetTextColor(hdc_in, pal.ink_muted);
            let mut r = RECT {
                left: content_x,
                top: s(y_dip),
                right: content_r,
                bottom: s(y_dip + 20),
            };
            let mut w: Vec<u16> = label.encode_utf16().collect();
            DrawTextW(hdc_in, &mut w, &mut r, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        };

        match cur_nav {
            0 => {
                draw_sec(mem_dc, "启动与系统主题", 60);
                draw_hairline(mem_dc, content_x, s(162), content_r, s(162), pal.line);
                draw_sec(mem_dc, "快捷功能", 172);
                draw_hairline(mem_dc, content_x, s(242), content_r, s(242), pal.line);
                draw_sec(mem_dc, "运行状态概览", 254);
            }
            1 => {
                draw_hairline(mem_dc, content_x, s(186), content_r, s(186), pal.line);
                draw_sec(mem_dc, "截图行为与保存位置", 198);
            }
            2 => {
                draw_hairline(mem_dc, content_x, s(128), content_r, s(128), pal.line);
                draw_sec(mem_dc, "显示内容与配色", 140);
                draw_hairline(mem_dc, content_x, s(304), content_r, s(304), pal.line);
                draw_sec(mem_dc, "任务栏芯片实时预览", 316);

                // Paint simulated Win11 taskbar background strip + live D2D frame
                let strip_rc = RECT {
                    left: content_x,
                    top: s(342),
                    right: content_r,
                    bottom: s(404),
                };
                let strip_bg = if light {
                    COLORREF(0x00EC_E8E4)
                } else {
                    COLORREF(0x0018_1818)
                };
                draw_rounded_rect(mem_dc, strip_rc, s(8), strip_bg, Some(pal.line));

                let mut preview_settings = ns;
                preview_settings.normalize();
                let pw = s(if !preview_settings.two_line && preview_settings.show_download && preview_settings.show_upload {
                    196
                } else {
                    148
                });
                let ph = s(44);
                let mut r = d2d.borrow_mut();
                if let Some(bytes) = r.render_netspeed_frame(
                    pw,
                    ph,
                    dpi,
                    !light,
                    theme_kind,
                    &preview_settings,
                    12_800_000.0,
                    1_420_000.0,
                    false,
                ) {
                    let bg_r = strip_bg.0 & 0xFF;
                    let bg_g = (strip_bg.0 >> 8) & 0xFF;
                    let bg_b = (strip_bg.0 >> 16) & 0xFF;
                    let mut composited = vec![0u8; (pw * ph * 4) as usize];
                    for (src, dst) in bytes.as_chunks::<4>().0.iter().zip(composited.as_chunks_mut::<4>().0) {
                        let a = u32::from(src[3]);
                        let inv = 255 - a;
                        dst[0] = (u32::from(src[0]) + ((bg_b * inv) / 255)).min(255) as u8;
                        dst[1] = (u32::from(src[1]) + ((bg_g * inv) / 255)).min(255) as u8;
                        dst[2] = (u32::from(src[2]) + ((bg_r * inv) / 255)).min(255) as u8;
                        dst[3] = 255;
                    }
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: pw,
                            biHeight: -ph,
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    let dx = strip_rc.left + ((strip_rc.right - strip_rc.left - pw) / 2);
                    let dy = strip_rc.top + ((strip_rc.bottom - strip_rc.top - ph) / 2);
                    let _ = SetDIBitsToDevice(
                        mem_dc,
                        dx,
                        dy,
                        pw as u32,
                        ph as u32,
                        0,
                        0,
                        0,
                        ph as u32,
                        composited.as_ptr().cast(),
                        &bmi,
                        DIB_RGB_COLORS,
                    );
                }
            }
            3 => {
                draw_hairline(mem_dc, content_x, s(240), content_r, s(240), pal.line);
                draw_sec(mem_dc, "应用状态与诊断提示", 254);
            }
            4 => {
                draw_sec(mem_dc, "日志与崩溃保护", 60);
                draw_hairline(mem_dc, content_x, s(212), content_r, s(212), pal.line);
                draw_sec(mem_dc, "版本信息与更新检查", 226);
            }
            _ => {}
        }

        SelectObject(mem_dc, old_f);
        let _ = BitBlt(hdc, 0, 0, cw, ch, Some(mem_dc), 0, 0, SRCCOPY);
        SelectObject(mem_dc, old_bmp);
        let _ = DeleteObject(mem_bmp.into());
        let _ = DeleteDC(mem_dc);
        let _ = EndPaint(hwnd, &ps);
    }
}

fn paint_owner_draw_button(dis: &DRAWITEMSTRUCT, state: &Rc<RefCell<SettingsState>>) {
    let id = dis.CtlID as usize;
    let (dpi, light, f_body, down_argb, up_argb, bg_argb, fx_argb_txt) = {
        let st = state.borrow();
        (
            st.dpi,
            st.light,
            st.font_body,
            st.down_argb,
            st.up_argb,
            st.bg_argb,
            get_text(st.edit_fx_argb),
        )
    };
    let pal = palette(light);
    let s = |dip: i32| s_px(dip, dpi);
    let selected = (dis.itemState.0 & ODS_SELECTED.0) != 0;
    let disabled = (dis.itemState.0 & ODS_DISABLED.0) != 0;
    let focused = (dis.itemState.0 & ODS_FOCUS.0) != 0;
    let is_primary = id == ID_BTN_TOGGLE_THEME || id == ID_BTN_OPEN_RELEASE;

    let (fill, border, txt_col) = if is_primary {
        let f = if selected {
            if light {
                COLORREF(0x009E_5200)
            } else {
                COLORREF(0x00DE_A338)
            }
        } else {
            pal.accent
        };
        let t = if light {
            COLORREF(0x00FF_FFFF)
        } else {
            COLORREF(0x0014_1414)
        };
        (f, None, t)
    } else {
        let f = if selected {
            pal.btn_pressed
        } else {
            pal.btn_bg
        };
        let b = if focused { Some(pal.accent) } else { Some(pal.btn_border) };
        let t = if disabled { pal.ink_muted } else { pal.ink };
        (f, b, t)
    };

    draw_rounded_rect(dis.hDC, dis.rcItem, s(6), fill, border);

    let swatch_col = match id {
        ID_BTN_BG_COLOR => Some(argb_to_colorref_local(bg_argb.unwrap_or(0xFF2B_2B2B))),
        ID_BTN_DOWN_COLOR => Some(argb_to_colorref_local(down_argb.unwrap_or(if light {
            0xFF08_6FA8
        } else {
            0xFF63_C7FF
        }))),
        ID_BTN_UP_COLOR => Some(argb_to_colorref_local(up_argb.unwrap_or(if light {
            0xFF16_803C
        } else {
            0xFF7E_E787
        }))),
        ID_BTN_FX_COLOR => {
            let argb = parse_argb_hex(&fx_argb_txt, TaskbarFxSettings::DEFAULT_ARGB);
            Some(argb_to_colorref_local(argb))
        }
        _ => None,
    };

    let mut text_rc = dis.rcItem;
    if let Some(sc) = swatch_col {
        let sw_sz = s(14);
        let sx = dis.rcItem.left + s(10);
        let sy = (dis.rcItem.top + dis.rcItem.bottom - sw_sz) / 2;
        draw_rounded_rect(
            dis.hDC,
            RECT {
                left: sx,
                top: sy,
                right: sx + sw_sz,
                bottom: sy + sw_sz,
            },
            s(3),
            sc,
            Some(pal.btn_border),
        );
        text_rc.left = sx + sw_sz + s(4);
    }

    let label = get_text(dis.hwndItem);
    let mut wide: Vec<u16> = label.encode_utf16().collect();
    unsafe {
        let old_f = SelectObject(dis.hDC, f_body.into());
        SetBkMode(dis.hDC, TRANSPARENT);
        SetTextColor(dis.hDC, txt_col);
        DrawTextW(
            dis.hDC,
            &mut wide,
            &mut text_rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
        SelectObject(dis.hDC, old_f);
    }
}

unsafe extern "system" fn settings_wnd_proc(
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
                let rc = Rc::from_raw(raw as *const RefCell<SettingsState>);
                {
                    let st = rc.borrow();
                    for br in [st.bg_brush, st.edit_brush] {
                        if !br.is_invalid() {
                            let _ = DeleteObject(br.into());
                        }
                    }
                    for f in [
                        st.font_body,
                        st.font_title,
                        st.font_section,
                        st.font_small,
                        st.font_nav_hdr,
                    ] {
                        if !f.is_invalid() {
                            let _ = DeleteObject(f.into());
                        }
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
        let r = Rc::from_raw(ptr as *const RefCell<SettingsState>);
        let c = Rc::clone(&r);
        std::mem::forget(r);
        c
    };

    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_settings_chrome(hwnd, &state_rc);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            let (dpi, prev_hover, tracking) = {
                let st = state_rc.borrow();
                (st.dpi, st.hover_nav, st.tracking_leave)
            };
            if !tracking {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe {
                    if TrackMouseEvent(&mut tme).is_ok() {
                        state_rc.borrow_mut().tracking_leave = true;
                    }
                }
            }
            let next_hover = hit_test_nav(x, y, dpi);
            if next_hover != prev_hover {
                state_rc.borrow_mut().hover_nav = next_hover;
                let s = |d: i32| s_px(d, dpi);
                let nav_rc = RECT {
                    left: 0,
                    top: 0,
                    right: s(NAV_W_DIP),
                    bottom: s(CLIENT_H_DIP),
                };
                unsafe {
                    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), Some(&nav_rc), false);
                }
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            let had_hover = {
                let mut st = state_rc.borrow_mut();
                st.tracking_leave = false;
                st.hover_nav.take().is_some()
            };
            if had_hover {
                invalidate_all(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            let (dpi, cur_nav) = {
                let st = state_rc.borrow();
                (st.dpi, st.current_nav)
            };
            if let Some(idx) = hit_test_nav(x, y, dpi) && idx != cur_nav {
                select_nav(&state_rc, idx);
            }
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
        WM_DRAWITEM => {
            let dis = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
            if dis.CtlType == ODT_BUTTON {
                paint_owner_draw_button(dis, &state_rc);
                return LRESULT(1);
            }
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let hdc = HDC(wparam.0 as *mut _);
            let ctrl = HWND(lparam.0 as *mut _);
            let (light, footer_bg, body_bg, bg_brush, status_lbl, conflict_lbl) = {
                let st = state_rc.borrow();
                let p = palette(st.light);
                (
                    st.light,
                    p.footer_bg,
                    p.body_bg,
                    st.bg_brush,
                    st.status_lbl,
                    st.hotkey_conflict_lbl,
                )
            };
            let p = palette(light);
            let is_footer = ctrl == status_lbl;
            let txt = if ctrl == conflict_lbl { p.err_ink } else { p.ink };
            unsafe {
                SetTextColor(hdc, txt);
                SetBkColor(hdc, if is_footer { footer_bg } else { body_bg });
            }
            if is_footer {
                let _ = footer_bg;
            }
            LRESULT(bg_brush.0 as isize)
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            let hdc = HDC(wparam.0 as *mut _);
            let st = state_rc.borrow();
            let p = palette(st.light);
            unsafe {
                SetTextColor(hdc, p.ink);
                SetBkColor(hdc, p.edit_bg);
            }
            LRESULT(st.edit_brush.0 as isize)
        }
        WM_HSCROLL => {
            let ctrl = HWND(lparam.0 as *mut _);
            let (sld_font, lbl_font, sld_alpha, lbl_alpha, edit_argb) = {
                let st = state_rc.borrow();
                (
                    st.sld_font,
                    st.lbl_font_val,
                    st.sld_fx_alpha,
                    st.lbl_fx_alpha,
                    st.edit_fx_argb,
                )
            };
            if ctrl == sld_font {
                let val = unsafe { SendMessageW(sld_font, TBM_GETPOS, None, None).0 };
                set_text(lbl_font, &format!("{val} px"));
                apply_now(&state_rc);
            } else if ctrl == sld_alpha {
                let alpha =
                    unsafe { SendMessageW(sld_alpha, TBM_GETPOS, None, None).0 }.clamp(0, 255) as u32;
                set_text(lbl_alpha, &alpha.to_string());
                let curr = parse_argb_hex(&get_text(edit_argb), TaskbarFxSettings::DEFAULT_ARGB);
                let next_argb = (curr & 0x00FF_FFFF) | (alpha << 24);
                set_text(edit_argb, &format!("#{next_argb:08X}"));
                apply_now(&state_rc);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = wparam.0 & 0xFFFF;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u32;
            let controller = state_rc.borrow().controller;

            match id {
                ID_BTN_CLOSE => unsafe {
                    let _ = DestroyWindow(hwnd);
                },
                ID_BTN_RESTORE => {
                    if confirm_warning(
                        hwnd,
                        "恢复默认设置？",
                        "网速外观、任务栏效果、截图选项等会回到默认。不会卸载 Suite，也不会删你已保存的截图文件。",
                    ) {
                        post_command(controller, AppCommand::RestoreDefaults, 0);
                    }
                }
                ID_BTN_TOGGLE_THEME => post_command(controller, AppCommand::ToggleTheme, 0),
                ID_BTN_QUICK_CAPTURE => post_command(controller, AppCommand::Capture, 0),
                ID_BTN_QUICK_DELAYED => post_command(controller, AppCommand::DelayedCapture, 0),
                ID_BTN_QUICK_PICK => post_command(controller, AppCommand::ColorPick, 0),
                ID_BTN_QUICK_SCROLL | ID_BTN_START_SCROLL => {
                    post_command(controller, AppCommand::ScrollCapture, 0)
                }
                ID_BTN_QUICK_PINS => post_command(controller, AppCommand::PinManage, 0),
                ID_BTN_OPEN_HISTORY => post_command(controller, AppCommand::History, 0),
                ID_BTN_TRY_NEXT_KEY => post_command(controller, AppCommand::TryNextHotkey, 0),
                ID_BTN_BROWSE_DIR => {
                    let cur_dir = get_text(state_rc.borrow().edit_save_dir);
                    let init = if cur_dir.trim().is_empty() {
                        SettingsPaths::default_capture_directory()
                            .to_string_lossy()
                            .into_owned()
                    } else {
                        cur_dir
                    };
                    if let Some(chosen) = pick_folder_dialog(hwnd, &init) {
                        let edit = state_rc.borrow().edit_save_dir;
                        set_text(edit, &chosen.to_string_lossy());
                        apply_now(&state_rc);
                    }
                }
                ID_BTN_OPEN_DIR => {
                    let cur = get_text(state_rc.borrow().edit_save_dir);
                    let dir = SettingsPaths::resolve_capture_directory(
                        (!cur.trim().is_empty()).then_some(cur.trim()),
                    );
                    let _ = std::fs::create_dir_all(&dir);
                    if !open_shell_target(&dir.to_string_lossy()) {
                        set_text(state_rc.borrow().status_lbl, "无法打开保存目录。");
                    }
                }
                ID_BTN_REFRESH_ADAPTER => {
                    post_command(controller, AppCommand::RefreshAdaptersAndSettings, 0);
                }
                ID_BTN_USE_ACRYLIC => post_command(controller, AppCommand::UseAcrylicReadable, 0),
                ID_BTN_OPEN_LOGS => {
                    if let Err(e) = SuiteLog::try_open_folder() {
                        set_text(state_rc.borrow().status_lbl, &e);
                    }
                }
                ID_BTN_CHECK_UPDATE => post_command(controller, AppCommand::CheckUpdates, 0),
                ID_BTN_OPEN_RELEASE => {
                    let url = state_rc
                        .borrow()
                        .download_url
                        .clone()
                        .unwrap_or_else(|| "https://github.com/tsumon/suite/releases".to_string());
                    let _ = open_shell_target(&url);
                }
                ID_BTN_GITHUB => {
                    let _ = open_shell_target("https://github.com/tsumon/suite");
                }
                ID_BTN_BG_COLOR => {
                    let cur = state_rc.borrow().bg_argb.unwrap_or(0xFF2B_2B2B);
                    if let Some(c) = choose_color_dialog(hwnd, cur) {
                        let btn = {
                            let mut st = state_rc.borrow_mut();
                            st.bg_argb = Some(c);
                            st.btn_bg_color
                        };
                        apply_now(&state_rc);
                        invalidate_all(btn);
                    }
                }
                ID_BTN_DOWN_COLOR => {
                    let cur = state_rc.borrow().down_argb.unwrap_or(0xFF1A_1A1A);
                    if let Some(c) = choose_color_dialog(hwnd, cur) {
                        let btn = {
                            let mut st = state_rc.borrow_mut();
                            st.down_argb = Some(c);
                            st.btn_down_color
                        };
                        apply_now(&state_rc);
                        invalidate_all(btn);
                    }
                }
                ID_BTN_DOWN_RESET => {
                    let btn = {
                        let mut st = state_rc.borrow_mut();
                        st.down_argb = None;
                        st.btn_down_color
                    };
                    apply_now(&state_rc);
                    invalidate_all(btn);
                }
                ID_BTN_UP_COLOR => {
                    let cur = state_rc.borrow().up_argb.unwrap_or(0xFF1A_1A1A);
                    if let Some(c) = choose_color_dialog(hwnd, cur) {
                        let btn = {
                            let mut st = state_rc.borrow_mut();
                            st.up_argb = Some(c);
                            st.btn_up_color
                        };
                        apply_now(&state_rc);
                        invalidate_all(btn);
                    }
                }
                ID_BTN_UP_RESET => {
                    let btn = {
                        let mut st = state_rc.borrow_mut();
                        st.up_argb = None;
                        st.btn_up_color
                    };
                    apply_now(&state_rc);
                    invalidate_all(btn);
                }
                ID_BTN_FX_COLOR => {
                    let (edit, sld, btn) = {
                        let st = state_rc.borrow();
                        (st.edit_fx_argb, st.sld_fx_alpha, st.btn_fx_color)
                    };
                    let cur = parse_argb_hex(&get_text(edit), TaskbarFxSettings::DEFAULT_ARGB);
                    if let Some(rgb) = choose_color_dialog(hwnd, cur) {
                        let alpha =
                            unsafe { SendMessageW(sld, TBM_GETPOS, None, None).0 }.clamp(0, 255)
                                as u32;
                        let argb = (alpha << 24) | (rgb & 0x00FF_FFFF);
                        set_text(edit, &format!("#{argb:08X}"));
                        apply_now(&state_rc);
                        invalidate_all(btn);
                    }
                }
                ID_CHK_SHOW_DOWN | ID_CHK_SHOW_UP => {
                    let (sd, su, slbl) = {
                        let st = state_rc.borrow();
                        (st.chk_show_down, st.chk_show_up, st.status_lbl)
                    };
                    if !is_checked(sd) && !is_checked(su) {
                        set_check(if id == ID_CHK_SHOW_DOWN { sd } else { su }, true);
                        set_text(slbl, NetSpeedCopy::AT_LEAST_ONE_LINE);
                        return LRESULT(0);
                    }
                    apply_now(&state_rc);
                }
                ID_CHK_LOGGING => {
                    let chk = state_rc.borrow().chk_logging;
                    if is_checked(chk) && let Err(e) = SuiteLog::try_write_probe() {
                        set_check(chk, false);
                        set_text(state_rc.borrow().status_lbl, &e);
                        return LRESULT(0);
                    }
                    apply_now(&state_rc);
                }
                ID_CMB_FX_MODE if code == CBN_SELCHANGE => {
                    let mode_idx = unsafe {
                        SendMessageW(state_rc.borrow().cmb_fx_mode, CB_GETCURSEL, None, None).0
                    };
                    let wire = match mode_idx {
                        0 => TaskbarAppearanceModes::NORMAL,
                        1 => TaskbarAppearanceModes::OPAQUE,
                        2 => TaskbarAppearanceModes::CLEAR,
                        _ => TaskbarAppearanceModes::ACRYLIC,
                    };
                    set_text(
                        state_rc.borrow().lbl_fx_mode_help,
                        TaskbarFxCopy::mode_help(Some(wire)),
                    );
                    refresh_light_tip(&state_rc);
                    apply_now(&state_rc);
                }
                ID_EDIT_SAVE_DIR => {
                    if code == EN_KILLFOCUS {
                        apply_now(&state_rc);
                    }
                }
                ID_EDIT_FX_ARGB => {
                    if code == EN_KILLFOCUS {
                        apply_now(&state_rc);
                        let btn = state_rc.borrow().btn_fx_color;
                        invalidate_all(btn);
                    }
                }
                ID_CMB_CAP_KEY | ID_CMB_PICK_KEY | ID_CMB_SCROLL_KEY | ID_CMB_CLICK_THROUGH
                | ID_CMB_HIST_MAX | ID_CMB_ADAPTER | ID_CMB_CHANNEL => {
                    if code == CBN_SELCHANGE {
                        apply_now(&state_rc);
                    }
                }
                ID_CHK_RUN_STARTUP | ID_CHK_CTRL | ID_CHK_SHIFT | ID_CHK_ALT | ID_CHK_WIN
                | ID_CHK_MAGNIFIER | ID_CHK_SAVE_FILE | ID_CHK_PIN_AFTER | ID_CHK_HISTORY
                | ID_CHK_NET_VISIBLE | ID_CHK_NET_EMBED | ID_CHK_NET_SEC | ID_CHK_TWO_LINE
                | ID_CHK_BOLD | ID_CHK_BG_TRANS | ID_CHK_BORDER | ID_CHK_FX_ENABLED
                | ID_CHK_FX_KEEP | ID_CHK_FX_AUTOOFF => {
                    refresh_light_tip(&state_rc);
                    apply_now(&state_rc);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let controller = state_rc.borrow().controller;
            unsafe {
                let _ = PostMessageW(Some(controller), WM_APP_SETTINGS_CLOSED, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

#[allow(dead_code)]
fn _keep_ns_type(_: NetSpeedSettings, _: &Path) {}
