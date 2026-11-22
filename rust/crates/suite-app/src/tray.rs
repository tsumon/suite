use suite_contracts::AppSettings;
use suite_platform::constants::to_wide_null;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIcon, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos, HICON,
    MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, SetForegroundWindow,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON, TrackPopupMenu,
};
use windows::core::PCWSTR;

use crate::messages::{AppCommand, WM_APP_TRAY_CALLBACK, post_command};
use crate::win32_util::hinstance;

pub struct TrayIconService {
    owner: HWND,
    icon: HICON,
    added: bool,
    net_visible: bool,
    start_with_windows: bool,
    scroll_label: String,
    tip: String,
    click_through_available: bool,
    pin_count: usize,
    pins_hidden: bool,
}

#[derive(Clone)]
pub struct TrayMenuArgs {
    pub owner: HWND,
    pub net_visible: bool,
    pub start_with_windows: bool,
    pub scroll_label: String,
    pub click_through_available: bool,
    pub pin_count: usize,
    pub pins_hidden: bool,
}

impl TrayIconService {
    const TRAY_ID: u32 = 1;

    pub fn new(owner: HWND) -> Self {
        let icon = create_suite_icon(16);
        let mut svc = Self {
            owner,
            icon,
            added: false,
            net_visible: true,
            start_with_windows: false,
            scroll_label: "滚动长截图 (F4)".to_string(),
            tip: "Suite".to_string(),
            click_through_available: false,
            pin_count: 0,
            pins_hidden: false,
        };
        svc.add_or_restore();
        svc
    }

    pub fn add_or_restore(&mut self) {
        if self.owner.is_invalid() {
            return;
        }
        let mut nid = self.base_nid();
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_APP_TRAY_CALLBACK;
        nid.hIcon = self.icon;
        write_wstr(&mut nid.szTip, &self.tip);
        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
        if ok {
            self.added = true;
            nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            unsafe {
                let _ = Shell_NotifyIconW(NIM_SETVERSION, &nid);
            }
        }
    }

    pub fn sync(&mut self, settings: &AppSettings) {
        self.net_visible = settings.net_speed.visible;
        self.start_with_windows = settings.start_with_windows;
        let scroll = &settings.capture.scroll_capture_hotkey;
        self.scroll_label = if scroll.is_disabled() {
            "滚动长截图".to_string()
        } else {
            format!("滚动长截图 ({})", scroll.to_display_string())
        };

        let mut tip = String::from("Suite");
        if !settings.hotkey.is_disabled() {
            tip.push_str(" · 截图 ");
            tip.push_str(&settings.hotkey.to_display_string());
        }
        if !scroll.is_disabled() {
            tip.push_str(" · 滚动 ");
            tip.push_str(&scroll.to_display_string());
        }
        // Keep tip concise within NOTIFYICONDATAW szTip limit
        let truncated: String = tip.chars().take(63).collect();
        self.tip = truncated;

        if self.added && !self.owner.is_invalid() {
            let mut nid = self.base_nid();
            nid.uFlags = NIF_TIP;
            write_wstr(&mut nid.szTip, &self.tip);
            unsafe {
                let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
            }
        }
    }

    pub fn set_pin_state(&mut self, count: usize, pins_hidden: bool, click_through_available: bool) {
        self.pin_count = count;
        self.pins_hidden = pins_hidden;
        self.click_through_available = click_through_available;
    }

    pub fn balloon(&self, title: &str, text: &str) {
        if !self.added || self.owner.is_invalid() || text.trim().is_empty() {
            return;
        }
        let mut nid = self.base_nid();
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = NIIF_INFO;
        nid.Anonymous.uTimeout = 4000;
        write_wstr(&mut nid.szInfoTitle, title);
        write_wstr(&mut nid.szInfo, text);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    pub fn menu_args(&self) -> TrayMenuArgs {
        TrayMenuArgs {
            owner: self.owner,
            net_visible: self.net_visible,
            start_with_windows: self.start_with_windows,
            scroll_label: self.scroll_label.clone(),
            click_through_available: self.click_through_available,
            pin_count: self.pin_count,
            pins_hidden: self.pins_hidden,
        }
    }

    pub fn show_popup_menu(args: &TrayMenuArgs) {
        if args.owner.is_invalid() {
            return;
        }
        unsafe {
            let Ok(hmenu) = CreatePopupMenu() else {
                return;
            };
            let append_item = |id: AppCommand, text: &str, flags: u32| {
                let w = to_wide_null(text);
                let _ = AppendMenuW(
                    hmenu,
                    windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS(MF_STRING.0 | flags),
                    id as usize,
                    PCWSTR(w.as_ptr()),
                );
            };

            append_item(AppCommand::ToggleTheme, "切换系统深浅色", 0);
            append_item(
                AppCommand::ToggleNetSpeed,
                if args.net_visible { "隐藏网速" } else { "显示网速" },
                0,
            );
            append_item(AppCommand::Capture, "区域截图", 0);
            append_item(AppCommand::DelayedCapture, "延时 3 秒截图", 0);
            append_item(AppCommand::ScrollCapture, &args.scroll_label, 0);
            append_item(AppCommand::ColorPick, "取色", 0);
            append_item(AppCommand::History, "截图历史", 0);
            append_item(AppCommand::PinFromClipboard, "贴图（从剪贴板）", 0);
            append_item(AppCommand::PinManage, "贴图管理", 0);
            append_item(
                AppCommand::TogglePinsVisible,
                if args.pins_hidden { "显示全部贴图" } else { "隐藏全部贴图" },
                if args.pin_count > 0 { 0 } else { MF_GRAYED.0 },
            );
            append_item(
                AppCommand::ClearClickThrough,
                "取消贴图穿透",
                if args.click_through_available { 0 } else { MF_GRAYED.0 },
            );
            append_item(AppCommand::OpenSettings, "设置...", 0);
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
            append_item(
                AppCommand::ToggleRunAtStartup,
                "开机启动",
                if args.start_with_windows {
                    MF_CHECKED.0
                } else {
                    MF_UNCHECKED.0
                },
            );
            append_item(AppCommand::Exit, "退出", 0);

            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let _ = SetForegroundWindow(args.owner);
            let cmd_raw = TrackPopupMenu(
                hmenu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | windows::Win32::UI::WindowsAndMessaging::TPM_RETURNCMD,
                pt.x,
                pt.y,
                Some(0),
                args.owner,
                None,
            );
            let _ = DestroyMenu(hmenu);
            if cmd_raw.0 > 0
                && let Some(cmd) = AppCommand::from_wparam(windows::Win32::Foundation::WPARAM(cmd_raw.0 as usize))
            {
                post_command(args.owner, cmd, 0);
            }
        }
    }

    fn base_nid(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.owner,
            uID: Self::TRAY_ID,
            ..Default::default()
        }
    }
}

impl Drop for TrayIconService {
    fn drop(&mut self) {
        if self.added && !self.owner.is_invalid() {
            let nid = self.base_nid();
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
            self.added = false;
        }
        if !self.icon.is_invalid() {
            unsafe {
                let _ = DestroyIcon(self.icon);
            }
        }
    }
}

fn write_wstr(dst: &mut [u16], text: &str) {
    dst.fill(0);
    let max_chars = dst.len().saturating_sub(1);
    for (i, unit) in text.encode_utf16().take(max_chars).enumerate() {
        dst[i] = unit;
    }
}

/// Programmatic Suite icon matching the dark rounded square + crisp inner frame.
pub fn create_suite_icon(size: i32) -> windows::Win32::UI::WindowsAndMessaging::HICON {
    let w = size.max(16);
    let h = size.max(16);
    let mut color_bits = vec![0u8; (w * h * 4) as usize];
    let mut mask_bits = vec![0u8; (((w + 31) / 32) * 4 * h) as usize];

    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let margin = w / 8;
            let hole = w / 3;
            let in_white = x > margin && x < w - margin - 1 && y > margin && y < h - margin - 1;
            let in_hole = x >= hole && x < w - hole && y >= hole && y < h - hole;
            if in_white && !in_hole {
                color_bits[i] = 0xFF;     // B
                color_bits[i + 1] = 0xFF; // G
                color_bits[i + 2] = 0xFF; // R
                color_bits[i + 3] = 0xFF; // A
            } else {
                color_bits[i] = 40;
                color_bits[i + 1] = 36;
                color_bits[i + 2] = 36;
                color_bits[i + 3] = 0xFF;
            }
        }
    }
    let _ = &mut mask_bits;

    unsafe {
        CreateIcon(
            Some(hinstance()),
            w,
            h,
            1,
            32,
            mask_bits.as_ptr(),
            color_bits.as_ptr(),
        )
        .unwrap_or_default()
    }
}
