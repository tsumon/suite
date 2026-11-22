use suite_contracts::HotkeyBinding;
use suite_platform::constants::{
    HOTKEY_ID_CAPTURE, HOTKEY_ID_COLOR_PICK, HOTKEY_ID_PIN_CLICK_THROUGH,
    HOTKEY_ID_PIN_CLIPBOARD, HOTKEY_ID_SCROLL_CAPTURE,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey};

#[derive(Default)]
pub struct HotKeyHost {
    hwnd: HWND,
    pub is_capture_registered: bool,
    pub last_capture_win32_error: u32,
    pub last_pin_win32_error: u32,
    registered_ids: Vec<i32>,
}

#[derive(Default)]
pub struct HotkeyRegReport {
    pub capture_error: Option<String>,
    pub pin_error: Option<String>,
    pub color_pick_error: Option<String>,
    pub click_through_error: Option<String>,
    pub scroll_capture_error: Option<String>,
}

impl HotKeyHost {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            is_capture_registered: false,
            last_capture_win32_error: 0,
            last_pin_win32_error: 0,
            registered_ids: Vec::new(),
        }
    }

    pub fn unregister_all(&mut self) {
        if self.hwnd.is_invalid() {
            self.registered_ids.clear();
            self.is_capture_registered = false;
            return;
        }
        for id in self.registered_ids.drain(..) {
            unsafe {
                let _ = UnregisterHotKey(Some(self.hwnd), id);
            }
        }
        self.is_capture_registered = false;
    }

    pub fn register_all(
        &mut self,
        capture: &HotkeyBinding,
        color_pick: &HotkeyBinding,
        pin_click_through: &HotkeyBinding,
        scroll_capture: &HotkeyBinding,
    ) -> HotkeyRegReport {
        self.unregister_all();
        let mut report = HotkeyRegReport::default();
        if self.hwnd.is_invalid() {
            return report;
        }

        // Capture hotkey (default F1)
        if !capture.is_disabled() {
            let (ok, err) = self.try_register(HOTKEY_ID_CAPTURE, capture);
            self.is_capture_registered = ok;
            self.last_capture_win32_error = err;
            if !ok {
                report.capture_error = Some(format!("Win32 error {err}"));
            }
        } else {
            self.is_capture_registered = true;
            self.last_capture_win32_error = 0;
        }

        // Pin from clipboard (F3 unless already used as capture key)
        if capture.control
            || capture.shift
            || capture.alt
            || capture.win
            || capture.virtual_key != HotkeyBinding::PIN_CLIPBOARD_VIRTUAL_KEY
        {
            let pin_bind = HotkeyBinding {
                virtual_key: HotkeyBinding::PIN_CLIPBOARD_VIRTUAL_KEY,
                ..Default::default()
            };
            let (ok, err) = self.try_register(HOTKEY_ID_PIN_CLIPBOARD, &pin_bind);
            self.last_pin_win32_error = err;
            if !ok {
                report.pin_error = Some(format!("Win32 error {err}"));
            }
        }

        if !color_pick.is_disabled() && !same_binding(color_pick, capture) {
            let (ok, err) = self.try_register(HOTKEY_ID_COLOR_PICK, color_pick);
            if !ok {
                report.color_pick_error = Some(format!("Win32 error {err}"));
            }
        }

        if !pin_click_through.is_disabled() && !same_binding(pin_click_through, capture) {
            let (ok, err) = self.try_register(HOTKEY_ID_PIN_CLICK_THROUGH, pin_click_through);
            if !ok {
                report.click_through_error = Some(format!("Win32 error {err}"));
            }
        }

        if !scroll_capture.is_disabled()
            && !same_binding(scroll_capture, capture)
            && !same_binding(scroll_capture, color_pick)
        {
            let (ok, err) = self.try_register(HOTKEY_ID_SCROLL_CAPTURE, scroll_capture);
            if !ok {
                report.scroll_capture_error = Some(format!("Win32 error {err}"));
            }
        }

        report
    }

    fn try_register(&mut self, id: i32, binding: &HotkeyBinding) -> (bool, u32) {
        let mods = HOT_KEY_MODIFIERS(binding.to_native_modifiers(true));
        let vk = binding.virtual_key as u32;
        unsafe {
            match RegisterHotKey(Some(self.hwnd), id, mods, vk) {
                Ok(()) => {
                    self.registered_ids.push(id);
                    (true, 0)
                }
                Err(e) => (false, (e.code().0 as u32) & 0xFFFF),
            }
        }
    }

    pub fn suggest_alternates(&self, current: &HotkeyBinding, count: usize) -> Vec<HotkeyBinding> {
        let mut results = Vec::with_capacity(count);
        for candidate in candidate_pool(current) {
            if same_binding(&candidate, current) {
                continue;
            }
            if self.is_probe_available(&candidate) {
                results.push(candidate);
                if results.len() >= count {
                    break;
                }
            }
        }
        results
    }

    pub fn find_next_available_after(&self, current: &HotkeyBinding) -> Option<HotkeyBinding> {
        let pool = candidate_pool(current);
        let start_idx = pool
            .iter()
            .position(|c| same_binding(c, current))
            .map(|i| i + 1)
            .unwrap_or(0);
        for offset in 0..pool.len() {
            let candidate = &pool[(start_idx + offset) % pool.len()];
            if same_binding(candidate, current) {
                continue;
            }
            if self.is_probe_available(candidate) {
                return Some(candidate.clone());
            }
        }
        None
    }

    fn is_probe_available(&self, binding: &HotkeyBinding) -> bool {
        if self.hwnd.is_invalid() {
            return false;
        }
        const PROBE_ID: i32 = 99;
        let mods = HOT_KEY_MODIFIERS(binding.to_native_modifiers(true));
        let vk = binding.virtual_key as u32;
        unsafe {
            if RegisterHotKey(Some(self.hwnd), PROBE_ID, mods, vk).is_ok() {
                let _ = UnregisterHotKey(Some(self.hwnd), PROBE_ID);
                true
            } else {
                false
            }
        }
    }
}

impl Drop for HotKeyHost {
    fn drop(&mut self) {
        self.unregister_all();
    }
}

fn candidate_pool(current: &HotkeyBinding) -> Vec<HotkeyBinding> {
    let mut pool = Vec::new();
    for vk in 0x70..=0x7B {
        if vk == HotkeyBinding::PIN_CLIPBOARD_VIRTUAL_KEY {
            continue;
        }
        pool.push(HotkeyBinding {
            control: current.control,
            shift: current.shift,
            alt: current.alt,
            win: current.win,
            virtual_key: vk,
        });
    }
    for vk in [0x41, 0x51, 0x53, 0x58, 0x43] {
        pool.push(HotkeyBinding {
            control: true,
            shift: true,
            alt: false,
            win: false,
            virtual_key: vk,
        });
    }
    pool
}

pub fn same_binding(a: &HotkeyBinding, b: &HotkeyBinding) -> bool {
    a.control == b.control
        && a.shift == b.shift
        && a.alt == b.alt
        && a.win == b.win
        && a.virtual_key == b.virtual_key
}
