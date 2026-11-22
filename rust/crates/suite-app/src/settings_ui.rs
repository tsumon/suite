use std::cell::RefCell;
use std::rc::Rc;

use suite_contracts::{
    AppSettings, HotkeyBinding, HotkeyCopy, NetSpeedCopy, NetSpeedSettings, SettingsPaths,
    TaskbarAppearanceModes, TaskbarFxCopy, TaskbarFxSettings, ThemeKind, UpdateSettings,
};
use suite_netspeed::InterfaceSnapshot;
use suite_platform::{PersonalizeRegistry, SuiteLog, constants::to_wide_null};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, HBRUSH, HDC, HFONT, SetBkColor, SetTextColor,
};
use windows::Win32::UI::Controls::{TBM_SETPOS, TBM_SETRANGE, TBS_AUTOTICKS};
use windows::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BM_SETCHECK, BS_AUTOCHECKBOX, CB_ADDSTRING, CB_GETCURSEL, CB_RESETCONTENT,
    CB_SETCURSEL, CBN_SELCHANGE, CBS_DROPDOWNLIST, CreateWindowExW, DefWindowProcW, DestroyWindow,
    EN_KILLFOCUS, ES_AUTOHSCROLL, GetWindowLongPtrW, GetWindowTextW, GWLP_USERDATA, HMENU,
    PostMessageW, SW_HIDE, SW_SHOW, SendMessageW, SetForegroundWindow, SetWindowLongPtrW,
    SetWindowTextW, ShowWindow, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX,
    WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_HSCROLL, WM_NCDESTROY, WS_BORDER, WS_CHILD,
    WS_OVERLAPPED, WS_CAPTION, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::PCWSTR;

use crate::messages::{
    AppCommand, WM_APP_SETTINGS_APPLY, WM_APP_SETTINGS_CLOSED, post_boxed, post_command,
};
use crate::update_checker::UpdateChecker;
use crate::win32_util::{
    choose_color_dialog, confirm_warning, create_ui_font, hinstance, invalidate_all,
    register_window_class, set_control_font,
};

const SETTINGS_WND_CLASS: &str = "SuiteSettingsWnd";
const TBM_GETPOS: u32 = 0x0400;

const BST_CHECKED: usize = 1;
const BST_UNCHECKED: usize = 0;

// Control IDs
const ID_NAV_BASE: usize = 100; // 100..104
const ID_BTN_RESTORE: usize = 201;
const ID_BTN_CLOSE: usize = 202;

// General
const ID_CHK_RUN_STARTUP: usize = 301;
const ID_BTN_TOGGLE_THEME: usize = 302;

// Capture
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

// NetSpeed
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

// TaskbarFx
const ID_CHK_FX_ENABLED: usize = 601;
const ID_CMB_FX_MODE: usize = 602;
const ID_SLD_FX_ALPHA: usize = 603;
const ID_EDIT_FX_ARGB: usize = 604;
const ID_BTN_FX_COLOR: usize = 605;
const ID_CHK_FX_KEEP: usize = 606;
const ID_BTN_USE_ACRYLIC: usize = 607;

// Advanced
const ID_CHK_LOGGING: usize = 701;
const ID_BTN_OPEN_LOGS: usize = 702;
const ID_CHK_FX_AUTOOFF: usize = 703;
const ID_CMB_CHANNEL: usize = 704;
const ID_BTN_CHECK_UPDATE: usize = 705;

struct AdapterEntry {
    if_index: u32,
    alias: String,
    display: String,
}

struct SettingsState {
    hwnd: HWND,
    controller: HWND,
    loading: bool,
    light: bool,
    bg_brush: HBRUSH,
    nav_brush: HBRUSH,
    font: HFONT,
    font_bold: HFONT,

    current_nav: usize,
    pages: [Vec<HWND>; 5],
    nav_btns: [HWND; 5],

    status_lbl: HWND,
    fx_status_lbl: HWND,
    hotkey_conflict_lbl: HWND,
    try_next_btn: HWND,
    light_tip_lbl: HWND,
    use_acrylic_btn: HWND,

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
    lbl_click_through: HWND,
    chk_magnifier: HWND,
    chk_save_file: HWND,
    edit_save_dir: HWND,
    chk_pin_after: HWND,
    chk_history: HWND,

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
}

pub struct SettingsWindow {
    state: Rc<RefCell<SettingsState>>,
}

impl SettingsWindow {
    pub fn show_or_focus(
        existing: Option<&Self>,
        controller: HWND,
        settings: &AppSettings,
        adapters: &[InterfaceSnapshot],
        last_start_label: &str,
    ) -> Rc<Self> {
        if let Some(win) = existing {
            let hwnd = win.state.borrow().hwnd;
            if !hwnd.is_invalid() {
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
        let light = PersonalizeRegistry::try_read()
            .map(|p| p.apps_use_light)
            .unwrap_or(true);
        let bg_brush = unsafe {
            CreateSolidBrush(if light {
                COLORREF(0x00F3_F3F3)
            } else {
                COLORREF(0x002B_2B2B)
            })
        };
        let nav_brush = unsafe {
            CreateSolidBrush(if light {
                COLORREF(0x00E4_E4E4)
            } else {
                COLORREF(0x001F_1F1F)
            })
        };
        let font = create_ui_font(13, false, false, "Microsoft YaHei UI");
        let font_bold = create_ui_font(15, false, true, "Microsoft YaHei UI");

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

        let state = Rc::new(RefCell::new(SettingsState {
            hwnd: HWND::default(),
            controller,
            loading: true,
            light,
            bg_brush,
            nav_brush,
            font,
            font_bold,
            current_nav: 0,
            pages: Default::default(),
            nav_btns: [HWND::default(); 5],
            status_lbl: HWND::default(),
            fx_status_lbl: HWND::default(),
            hotkey_conflict_lbl: HWND::default(),
            try_next_btn: HWND::default(),
            light_tip_lbl: HWND::default(),
            use_acrylic_btn: HWND::default(),
            chk_run_startup: HWND::default(),
            lbl_general_summary: HWND::default(),
            chk_ctrl: HWND::default(),
            chk_shift: HWND::default(),
            chk_alt: HWND::default(),
            chk_win: HWND::default(),
            cmb_cap_key: HWND::default(),
            cmb_pick_key: HWND::default(),
            cmb_scroll_key: HWND::default(),
            lbl_click_through: HWND::default(),
            chk_magnifier: HWND::default(),
            chk_save_file: HWND::default(),
            edit_save_dir: HWND::default(),
            chk_pin_after: HWND::default(),
            chk_history: HWND::default(),
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
            down_argb: settings.net_speed.down_color_argb,
            up_argb: settings.net_speed.up_color_argb,
            bg_argb: settings.net_speed.background_argb,
            chk_fx_enabled: HWND::default(),
            cmb_fx_mode: HWND::default(),
            lbl_fx_mode_help: HWND::default(),
            sld_fx_alpha: HWND::default(),
            lbl_fx_alpha: HWND::default(),
            edit_fx_argb: HWND::default(),
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
        }));

        let cls = to_wide_null(SETTINGS_WND_CLASS);
        let title = to_wide_null(&format!("Suite 设置 — {}", UpdateChecker::LOCAL_VERSION));
        let raw = Rc::into_raw(Rc::clone(&state));
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(cls.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                140,
                100,
                740,
                550,
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
        build_settings_controls(hwnd, &state);
        let win = Rc::new(Self { state });
        win.reload(settings, adapters, last_start_label);
        select_nav(&win.state, 0);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
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
        self.state.borrow_mut().loading = false;
    }

    pub fn apply_theme(&self, theme: ThemeKind) {
        let light = theme == ThemeKind::Light;
        let hwnd = {
            let mut st = self.state.borrow_mut();
            st.light = light;
            if !st.bg_brush.is_invalid() {
                unsafe {
                    let _ = DeleteObject(st.bg_brush.into());
                }
            }
            if !st.nav_brush.is_invalid() {
                unsafe {
                    let _ = DeleteObject(st.nav_brush.into());
                }
            }
            st.bg_brush = unsafe {
                CreateSolidBrush(if light {
                    COLORREF(0x00F3_F3F3)
                } else {
                    COLORREF(0x002B_2B2B)
                })
            };
            st.nav_brush = unsafe {
                CreateSolidBrush(if light {
                    COLORREF(0x00E4_E4E4)
                } else {
                    COLORREF(0x001F_1F1F)
                })
            };
            st.hwnd
        };
        refresh_light_tip(&self.state);
        invalidate_all(hwnd);
    }

    pub fn set_status(&self, msg: &str) {
        let lbl = self.state.borrow().status_lbl;
        set_text(lbl, msg);
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
        let (lbl, btn) = {
            let st = self.state.borrow();
            (st.hotkey_conflict_lbl, st.try_next_btn)
        };
        set_text(lbl, msg);
        set_text(self.state.borrow().status_lbl, msg);
        unsafe {
            let _ = ShowWindow(lbl, SW_SHOW);
            let _ = ShowWindow(btn, SW_SHOW);
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
    let (font, font_bold, cap_keys, fn_keys, scroll_keys) = {
        let st = state.borrow();
        (
            st.font,
            st.font_bold,
            st.cap_keys.clone(),
            st.fn_keys.clone(),
            st.scroll_keys.clone(),
        )
    };
    let btn_cls = to_wide_null("BUTTON");
    let lbl_cls = to_wide_null("STATIC");
    let edit_cls = to_wide_null("EDIT");
    let cmb_cls = to_wide_null("COMBOBOX");
    let trk_cls = to_wide_null("msctls_trackbar32");

    let mk_lbl = |text: &str, x: i32, y: i32, w: i32, h: i32, bold: bool| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(lbl_cls.as_ptr()),
                PCWSTR(to_wide_null(text).as_ptr()),
                WS_CHILD | WS_VISIBLE,
                x,
                y,
                w,
                h,
                Some(hwnd),
                None,
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, if bold { font_bold } else { font });
        hw
    };
    let mk_chk = |text: &str, x: i32, y: i32, w: i32, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(btn_cls.as_ptr()),
                PCWSTR(to_wide_null(text).as_ptr()),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                x,
                y,
                w,
                22,
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, font);
        hw
    };
    let mk_btn = |text: &str, x: i32, y: i32, w: i32, h: i32, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(btn_cls.as_ptr()),
                PCWSTR(to_wide_null(text).as_ptr()),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                x,
                y,
                w,
                h,
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, font);
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
                    | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
                x,
                y,
                w,
                180,
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, font);
        hw
    };
    let mk_edit = |x: i32, y: i32, w: i32, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(edit_cls.as_ptr()),
                PCWSTR::null(),
                WS_CHILD
                    | WS_VISIBLE
                    | WS_BORDER
                    | WS_TABSTOP
                    | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                x,
                y,
                w,
                24,
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        set_control_font(hw, font);
        hw
    };
    let mk_slider = |x: i32, y: i32, w: i32, min: isize, max: isize, id: usize| -> HWND {
        let hw = unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(trk_cls.as_ptr()),
                PCWSTR::null(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(TBS_AUTOTICKS),
                x,
                y,
                w,
                26,
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(hinstance()),
                None,
            )
            .unwrap_or_default()
        };
        unsafe {
            let lparam = ((max as usize) << 16) | (min as usize & 0xFFFF);
            let _ = SendMessageW(hw, TBM_SETRANGE, Some(WPARAM(1)), Some(LPARAM(lparam as isize)));
        }
        hw
    };

    // Left Nav
    let nav_names = ["常规", "截图与热键", "任务栏网速", "任务栏透明", "高级与关于"];
    for (i, &name) in nav_names.iter().enumerate() {
        let b = mk_btn(name, 14, 18 + (i as i32) * 38, 130, 32, ID_NAV_BASE + i);
        state.borrow_mut().nav_btns[i] = b;
    }

    // Footer
    let status_lbl = mk_lbl("就绪", 160, 474, 340, 22, false);
    let _b_res = mk_btn("恢复默认", 514, 470, 96, 28, ID_BTN_RESTORE);
    let _b_cls = mk_btn("关闭", 618, 470, 88, 28, ID_BTN_CLOSE);

    let px = 166;
    // Page 0: General
    let mut p0 = Vec::new();
    p0.push(mk_lbl("常规", px, 16, 200, 24, true));
    let chk_run = mk_chk("开机启动（写入当前用户 Run 注册表）", px, 56, 360, ID_CHK_RUN_STARTUP);
    p0.push(chk_run);
    let b_theme = mk_btn("切换系统深浅色 (Apps + System)", px, 94, 260, 30, ID_BTN_TOGGLE_THEME);
    p0.push(b_theme);
    let lbl_sum = mk_lbl("", px, 140, 500, 60, false);
    p0.push(lbl_sum);
    p0.push(mk_lbl(
        "所有更改即时生效并自动保存至 settings.json。",
        px,
        215,
        460,
        22,
        false,
    ));

    // Page 1: Capture & Hotkeys
    let mut p1 = Vec::new();
    p1.push(mk_lbl("截图与热键", px, 16, 200, 24, true));
    p1.push(mk_lbl("区域截图热键：", px, 52, 110, 22, false));
    let chk_ctrl = mk_chk("Ctrl", px + 115, 50, 55, ID_CHK_CTRL);
    let chk_shift = mk_chk("Shift", px + 175, 50, 60, ID_CHK_SHIFT);
    let chk_alt = mk_chk("Alt", px + 240, 50, 50, ID_CHK_ALT);
    let chk_win = mk_chk("Win", px + 295, 50, 55, ID_CHK_WIN);
    let cmb_cap = mk_cmb(px + 358, 48, 86, ID_CMB_CAP_KEY);
    for (_, label) in &cap_keys {
        let w = to_wide_null(label);
        unsafe {
            let _ = SendMessageW(cmb_cap, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    p1.extend([chk_ctrl, chk_shift, chk_alt, chk_win, cmb_cap]);

    let lbl_conflict = mk_lbl("", px, 78, 390, 20, false);
    let btn_try_next = mk_btn(HotkeyCopy::TRY_NEXT_BUTTON, px + 398, 75, 130, 24, ID_BTN_TRY_NEXT_KEY);
    unsafe {
        let _ = ShowWindow(lbl_conflict, SW_HIDE);
        let _ = ShowWindow(btn_try_next, SW_HIDE);
    }
    p1.extend([lbl_conflict, btn_try_next]);

    p1.push(mk_lbl("取色热键：", px, 108, 100, 22, false));
    let cmb_pick = mk_cmb(px + 115, 105, 86, ID_CMB_PICK_KEY);
    for (_, label) in &fn_keys {
        let w = to_wide_null(label);
        unsafe {
            let _ = SendMessageW(cmb_pick, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    p1.push(cmb_pick);

    p1.push(mk_lbl("滚动长截图：", px, 140, 100, 22, false));
    let cmb_scroll = mk_cmb(px + 115, 137, 160, ID_CMB_SCROLL_KEY);
    for (_, label) in &scroll_keys {
        let w = to_wide_null(label);
        unsafe {
            let _ = SendMessageW(cmb_scroll, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    let btn_scroll_now = mk_btn("立即滚动截图", px + 286, 136, 120, 25, ID_BTN_START_SCROLL);
    p1.extend([cmb_scroll, btn_scroll_now]);

    p1.push(mk_lbl("贴图穿透热键：", px, 174, 110, 22, false));
    let lbl_ct = mk_lbl("Ctrl+Shift+Q", px + 115, 174, 180, 22, false);
    p1.push(lbl_ct);

    let chk_mag = mk_chk("选区时显示放大镜", px, 208, 220, ID_CHK_MAGNIFIER);
    let chk_pin_after = mk_chk("截图后直接钉图（跳过工具栏）", px + 230, 208, 250, ID_CHK_PIN_AFTER);
    let chk_save = mk_chk("截图后自动保存到文件", px, 240, 220, ID_CHK_SAVE_FILE);
    let chk_hist = mk_chk("保留最近截图历史", px + 230, 240, 180, ID_CHK_HISTORY);
    let btn_hist = mk_btn("查看历史...", px + 418, 238, 100, 25, ID_BTN_OPEN_HISTORY);
    p1.extend([chk_mag, chk_pin_after, chk_save, chk_hist, btn_hist]);

    p1.push(mk_lbl("保存目录：", px, 276, 80, 22, false));
    let edit_dir = mk_edit(px + 85, 273, 430, ID_EDIT_SAVE_DIR);
    p1.push(edit_dir);
    p1.push(mk_lbl(
        &format!("留空则保存到：{}", SettingsPaths::default_capture_directory().display()),
        px,
        304,
        520,
        20,
        false,
    ));

    // Page 2: NetSpeed
    let mut p2 = Vec::new();
    p2.push(mk_lbl("任务栏网速", px, 16, 200, 24, true));
    let chk_nv = mk_chk("显示网速", px, 50, 110, ID_CHK_NET_VISIBLE);
    let chk_ne = mk_chk("嵌入主任务栏", px + 125, 50, 140, ID_CHK_NET_EMBED);
    let chk_ns = mk_chk("同时嵌入副屏任务栏", px + 275, 50, 190, ID_CHK_NET_SEC);
    p2.extend([chk_nv, chk_ne, chk_ns]);

    p2.push(mk_lbl("网卡：", px, 86, 56, 22, false));
    let cmb_ad = mk_cmb(px + 60, 83, 360, ID_CMB_ADAPTER);
    let btn_ref_ad = mk_btn("刷新", px + 430, 82, 76, 25, ID_BTN_REFRESH_ADAPTER);
    p2.extend([cmb_ad, btn_ref_ad]);

    p2.push(mk_lbl("字号：", px, 122, 56, 22, false));
    let sld_font = mk_slider(px + 60, 118, 220, 10, 18, ID_SLD_FONT);
    let lbl_font_val = mk_lbl("13", px + 290, 122, 40, 22, false);
    p2.extend([sld_font, lbl_font_val]);

    let chk_sd = mk_chk("显示下行", px, 156, 110, ID_CHK_SHOW_DOWN);
    let chk_su = mk_chk("显示上行", px + 125, 156, 110, ID_CHK_SHOW_UP);
    let chk_tl = mk_chk("双行显示", px + 250, 156, 110, ID_CHK_TWO_LINE);
    let chk_bd = mk_chk("粗体", px + 370, 156, 80, ID_CHK_BOLD);
    p2.extend([chk_sd, chk_su, chk_tl, chk_bd]);

    let chk_bgt = mk_chk("背景透明", px, 192, 110, ID_CHK_BG_TRANS);
    let btn_bgc = mk_btn("背景色...", px + 125, 189, 96, 26, ID_BTN_BG_COLOR);
    let chk_bdr = mk_chk("显示边框", px + 240, 192, 110, ID_CHK_BORDER);
    p2.extend([chk_bgt, btn_bgc, chk_bdr]);

    let btn_dc = mk_btn("下行颜色...", px, 232, 100, 26, ID_BTN_DOWN_COLOR);
    let btn_dr = mk_btn("重置", px + 106, 232, 60, 26, ID_BTN_DOWN_RESET);
    let btn_uc = mk_btn("上行颜色...", px + 186, 232, 100, 26, ID_BTN_UP_COLOR);
    let btn_ur = mk_btn("重置", px + 292, 232, 60, 26, ID_BTN_UP_RESET);
    p2.extend([btn_dc, btn_dr, btn_uc, btn_ur]);
    p2.push(mk_lbl(NetSpeedCopy::EMBED_HELP, px, 276, 520, 40, false));

    // Page 3: TaskbarFx
    let mut p3 = Vec::new();
    p3.push(mk_lbl("任务栏透明", px, 16, 200, 24, true));
    let chk_fx_on = mk_chk("启用任务栏透明", px, 50, 180, ID_CHK_FX_ENABLED);
    p3.push(chk_fx_on);

    p3.push(mk_lbl("模式：", px, 86, 60, 22, false));
    let cmb_fx_m = mk_cmb(px + 65, 83, 160, ID_CMB_FX_MODE);
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
    let lbl_fx_m_help = mk_lbl("", px + 236, 86, 280, 22, false);
    p3.extend([cmb_fx_m, lbl_fx_m_help]);

    p3.push(mk_lbl("透明度：", px, 124, 65, 22, false));
    let sld_alpha = mk_slider(px + 65, 120, 220, 0, 255, ID_SLD_FX_ALPHA);
    let lbl_alpha = mk_lbl("204", px + 295, 124, 40, 22, false);
    p3.extend([sld_alpha, lbl_alpha]);

    p3.push(mk_lbl("ARGB 颜色：", px, 162, 85, 22, false));
    let edit_argb = mk_edit(px + 90, 159, 130, ID_EDIT_FX_ARGB);
    let btn_fx_col = mk_btn("选色...", px + 230, 158, 86, 26, ID_BTN_FX_COLOR);
    p3.extend([edit_argb, btn_fx_col]);

    let chk_fx_keep = mk_chk("Suite 退出后保持任务栏外观", px, 198, 280, ID_CHK_FX_KEEP);
    let lbl_fx_status = mk_lbl("", px, 232, 520, 24, false);
    let lbl_light_tip = mk_lbl(
        "浅色主题下，透明任务栏上的网速可能看不清。可关「背景透明」或改用亚克力方案。",
        px,
        266,
        520,
        36,
        false,
    );
    let btn_use_acr = mk_btn("改用亚克力方案", px, 306, 150, 28, ID_BTN_USE_ACRYLIC);
    p3.extend([chk_fx_keep, lbl_fx_status, lbl_light_tip, btn_use_acr]);

    // Page 4: Advanced & About
    let mut p4 = Vec::new();
    p4.push(mk_lbl("高级与关于", px, 16, 200, 24, true));
    let chk_log = mk_chk("记录调试日志", px, 52, 160, ID_CHK_LOGGING);
    let btn_logs = mk_btn("打开日志文件夹", px + 175, 49, 140, 26, ID_BTN_OPEN_LOGS);
    p4.extend([chk_log, btn_logs]);
    p4.push(mk_lbl(
        &format!("日志目录：{}", SettingsPaths::logs_directory().display()),
        px,
        82,
        520,
        20,
        false,
    ));

    let chk_autooff = mk_chk("任务栏透明异常退出后自动关闭效果（保住截图与网速）", px, 114, 440, ID_CHK_FX_AUTOOFF);
    let lbl_last = mk_lbl("上次启动：正常", px, 148, 240, 22, false);
    p4.extend([chk_autooff, lbl_last]);

    p4.push(mk_lbl("更新通道：", px, 186, 80, 22, false));
    let cmb_chan = mk_cmb(px + 85, 183, 120, ID_CMB_CHANNEL);
    for ch in ["正式", "预览"] {
        let w = to_wide_null(ch);
        unsafe {
            let _ = SendMessageW(cmb_chan, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
    let btn_upd = mk_btn("检查更新", px + 220, 182, 110, 26, ID_BTN_CHECK_UPDATE);
    p4.extend([cmb_chan, btn_upd]);
    p4.push(mk_lbl(
        &format!("当前版本：{} (Rust Native)", UpdateChecker::LOCAL_VERSION),
        px,
        228,
        400,
        22,
        false,
    ));

    let mut st = state.borrow_mut();
    st.status_lbl = status_lbl;
    st.fx_status_lbl = lbl_fx_status;
    st.hotkey_conflict_lbl = lbl_conflict;
    st.try_next_btn = btn_try_next;
    st.light_tip_lbl = lbl_light_tip;
    st.use_acrylic_btn = btn_use_acr;
    st.chk_run_startup = chk_run;
    st.lbl_general_summary = lbl_sum;
    st.chk_ctrl = chk_ctrl;
    st.chk_shift = chk_shift;
    st.chk_alt = chk_alt;
    st.chk_win = chk_win;
    st.cmb_cap_key = cmb_cap;
    st.cmb_pick_key = cmb_pick;
    st.cmb_scroll_key = cmb_scroll;
    st.lbl_click_through = lbl_ct;
    st.chk_magnifier = chk_mag;
    st.chk_save_file = chk_save;
    st.edit_save_dir = edit_dir;
    st.chk_pin_after = chk_pin_after;
    st.chk_history = chk_hist;
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
    st.chk_fx_enabled = chk_fx_on;
    st.cmb_fx_mode = cmb_fx_m;
    st.lbl_fx_mode_help = lbl_fx_m_help;
    st.sld_fx_alpha = sld_alpha;
    st.lbl_fx_alpha = lbl_alpha;
    st.edit_fx_argb = edit_argb;
    st.chk_fx_keep = chk_fx_keep;
    st.chk_logging = chk_log;
    st.chk_fx_autooff = chk_autooff;
    st.lbl_last_start = lbl_last;
    st.cmb_channel = cmb_chan;
    st.pages = [p0, p1, p2, p3, p4];
}

fn select_nav(state: &Rc<RefCell<SettingsState>>, index: usize) {
    let pages = {
        let mut st = state.borrow_mut();
        st.current_nav = index;
        st.pages.clone()
    };
    for (i, group) in pages.iter().enumerate() {
        let cmd = if i == index { SW_SHOW } else { SW_HIDE };
        for &h in group {
            unsafe {
                let _ = ShowWindow(h, cmd);
            }
        }
    }
    if index == 3 {
        refresh_light_tip(state);
    }
}

fn populate_controls_from_state(state: &Rc<RefCell<SettingsState>>, last_start_label: &str) {
    let st = state.borrow();
    let s = &st.settings_snapshot;

    set_check(st.chk_run_startup, s.start_with_windows);
    let summary = format!(
        "截图快捷键：{}\n网速：{} · 透明：{}",
        if s.hotkey.is_disabled() {
            "关闭".to_string()
        } else {
            s.hotkey.to_display_string()
        },
        if s.net_speed.visible { "开" } else { "关" },
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
    set_text(st.lbl_click_through, &s.capture.pin_click_through_hotkey.to_display_string());
    set_check(st.chk_magnifier, s.capture.show_magnifier);
    set_check(st.chk_save_file, s.capture.save_file_after_capture);
    set_text(st.edit_save_dir, &s.capture.save_directory);
    set_check(st.chk_pin_after, s.capture.pin_after_capture);
    set_check(st.chk_history, s.capture.history_enabled);

    set_check(st.chk_net_vis, s.net_speed.visible);
    set_check(st.chk_net_embed, s.net_speed.embed_in_taskbar);
    set_check(st.chk_net_sec, s.net_speed.embed_secondary);
    unsafe {
        let _ = SendMessageW(st.cmb_adapter, CB_RESETCONTENT, None, None);
        for a in &st.adapters {
            let w = to_wide_null(&a.display);
            let _ = SendMessageW(st.cmb_adapter, CB_ADDSTRING, None, Some(LPARAM(w.as_ptr() as isize)));
        }
        let ad_idx = s
            .net_speed
            .if_index
            .and_then(|idx| st.adapters.iter().position(|a| a.if_index == idx))
            .unwrap_or(0);
        let _ = SendMessageW(st.cmb_adapter, CB_SETCURSEL, Some(WPARAM(ad_idx)), Some(LPARAM(0)));
    }
    let fs = s.net_speed.font_size_dip.round().clamp(10.0, 18.0) as isize;
    unsafe {
        let _ = SendMessageW(st.sld_font, TBM_SETPOS, Some(WPARAM(1)), Some(LPARAM(fs)));
    }
    set_text(st.lbl_font_val, &fs.to_string());
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
    next.capture.show_magnifier = is_checked(st.chk_magnifier);
    next.capture.save_file_after_capture = is_checked(st.chk_save_file);
    next.capture.save_directory = get_text(st.edit_save_dir).trim().to_string();
    next.capture.pin_after_capture = is_checked(st.chk_pin_after);
    next.capture.history_enabled = is_checked(st.chk_history);

    next.net_speed.visible = is_checked(st.chk_net_vis);
    next.net_speed.embed_in_taskbar = is_checked(st.chk_net_embed);
    next.net_speed.embed_secondary = is_checked(st.chk_net_sec);
    let ad_sel = unsafe { SendMessageW(st.cmb_adapter, CB_GETCURSEL, None, None).0 };
    if let Some(a) = st.adapters.get(ad_sel.max(0) as usize) {
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
    let (controller, status_lbl) = {
        let mut st = state.borrow_mut();
        st.settings_snapshot = next.clone();
        (st.controller, st.status_lbl)
    };
    set_text(status_lbl, "已保存");
    post_boxed(controller, WM_APP_SETTINGS_APPLY, 0, next);
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
                    if !st.bg_brush.is_invalid() {
                        let _ = DeleteObject(st.bg_brush.into());
                    }
                    if !st.nav_brush.is_invalid() {
                        let _ = DeleteObject(st.nav_brush.into());
                    }
                    if !st.font.is_invalid() {
                        let _ = DeleteObject(st.font.into());
                    }
                    if !st.font_bold.is_invalid() {
                        let _ = DeleteObject(st.font_bold.into());
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
        WM_ERASEBKGND => {
            let hdc = HDC(wparam.0 as *mut _);
            let st = state_rc.borrow();
            let rc = crate::win32_util::client_rect(hwnd);
            let nav_rc = windows::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 154,
                bottom: rc.bottom,
            };
            let body_rc = windows::Win32::Foundation::RECT {
                left: 154,
                top: 0,
                right: rc.right,
                bottom: rc.bottom,
            };
            unsafe {
                windows::Win32::Graphics::Gdi::FillRect(hdc, &nav_rc, st.nav_brush);
                windows::Win32::Graphics::Gdi::FillRect(hdc, &body_rc, st.bg_brush);
            }
            LRESULT(1)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let hdc = HDC(wparam.0 as *mut _);
            let st = state_rc.borrow();
            unsafe {
                SetTextColor(
                    hdc,
                    if st.light { COLORREF(0x0017_1717) } else { COLORREF(0x00F5_F5F5) },
                );
                SetBkColor(
                    hdc,
                    if st.light { COLORREF(0x00F3_F3F3) } else { COLORREF(0x002B_2B2B) },
                );
            }
            LRESULT(st.bg_brush.0 as isize)
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            let hdc = HDC(wparam.0 as *mut _);
            let st = state_rc.borrow();
            unsafe {
                SetTextColor(
                    hdc,
                    if st.light { COLORREF(0x0017_1717) } else { COLORREF(0x00F5_F5F5) },
                );
                SetBkColor(
                    hdc,
                    if st.light { COLORREF(0x00FF_FFFF) } else { COLORREF(0x0038_3838) },
                );
            }
            LRESULT(st.bg_brush.0 as isize)
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
                set_text(lbl_font, &val.to_string());
                apply_now(&state_rc);
            } else if ctrl == sld_alpha {
                let alpha = unsafe { SendMessageW(sld_alpha, TBM_GETPOS, None, None).0 }.clamp(0, 255) as u32;
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

            if (ID_NAV_BASE..ID_NAV_BASE + 5).contains(&id) {
                select_nav(&state_rc, id - ID_NAV_BASE);
                return LRESULT(0);
            }

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
                ID_BTN_TRY_NEXT_KEY => post_command(controller, AppCommand::TryNextHotkey, 0),
                ID_BTN_START_SCROLL => post_command(controller, AppCommand::ScrollCapture, 0),
                ID_BTN_OPEN_HISTORY => post_command(controller, AppCommand::History, 0),
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
                ID_BTN_BG_COLOR => {
                    let cur = state_rc.borrow().bg_argb.unwrap_or(0xFF2B_2B2B);
                    if let Some(c) = choose_color_dialog(hwnd, cur) {
                        state_rc.borrow_mut().bg_argb = Some(c);
                        apply_now(&state_rc);
                    }
                }
                ID_BTN_DOWN_COLOR => {
                    let cur = state_rc.borrow().down_argb.unwrap_or(0xFF1A_1A1A);
                    if let Some(c) = choose_color_dialog(hwnd, cur) {
                        state_rc.borrow_mut().down_argb = Some(c);
                        apply_now(&state_rc);
                    }
                }
                ID_BTN_DOWN_RESET => {
                    state_rc.borrow_mut().down_argb = None;
                    apply_now(&state_rc);
                }
                ID_BTN_UP_COLOR => {
                    let cur = state_rc.borrow().up_argb.unwrap_or(0xFF1A_1A1A);
                    if let Some(c) = choose_color_dialog(hwnd, cur) {
                        state_rc.borrow_mut().up_argb = Some(c);
                        apply_now(&state_rc);
                    }
                }
                ID_BTN_UP_RESET => {
                    state_rc.borrow_mut().up_argb = None;
                    apply_now(&state_rc);
                }
                ID_BTN_FX_COLOR => {
                    let edit = state_rc.borrow().edit_fx_argb;
                    let sld = state_rc.borrow().sld_fx_alpha;
                    let cur = parse_argb_hex(&get_text(edit), TaskbarFxSettings::DEFAULT_ARGB);
                    if let Some(rgb) = choose_color_dialog(hwnd, cur) {
                        let alpha = unsafe { SendMessageW(sld, TBM_GETPOS, None, None).0 }.clamp(0, 255) as u32;
                        let argb = (alpha << 24) | (rgb & 0x00FF_FFFF);
                        set_text(edit, &format!("#{argb:08X}"));
                        apply_now(&state_rc);
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
                    set_text(state_rc.borrow().lbl_fx_mode_help, TaskbarFxCopy::mode_help(Some(wire)));
                    refresh_light_tip(&state_rc);
                    apply_now(&state_rc);
                }
                ID_EDIT_SAVE_DIR | ID_EDIT_FX_ARGB => {
                    if code == EN_KILLFOCUS {
                        apply_now(&state_rc);
                    }
                }
                ID_CMB_CAP_KEY | ID_CMB_PICK_KEY | ID_CMB_SCROLL_KEY | ID_CMB_ADAPTER
                | ID_CMB_CHANNEL => {
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
fn _unused_ns(_: NetSpeedSettings) {}
