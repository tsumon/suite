use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use suite_capture::{
    ColorPick, ImageClipboard, MonitorEnumerator, PixelRect, ScreenshotHistory, ScrollCapture,
    WindowsOcr, save_png_atomic,
};
use suite_contracts::{
    AppSettings, HotkeyCopy, NetSpeedCopy, TaskbarAppearanceModes, TaskbarFxCopy,
    TaskbarFxSettings, ThemeKind,
};
use suite_netspeed::{IpHelperInterfaceTable, NetSpeedSampler, RateSample};
use suite_platform::{
    CrashGuard, PersonalizeRegistry, RunKeyService, SettingsStore, SuiteLog, TaskbarKind,
    TaskbarProbe, constants::to_wide_null,
};
use suite_taskbarfx::TaskbarFxEngine;
use suite_theme::{NativeThemeBroadcaster, RegistryPersonalizeStore, ThemeToggleService};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, GWLP_USERDATA, KillTimer,
    PostMessageW, PostQuitMessage, RegisterWindowMessageW, SetTimer, SetWindowLongPtrW,
    WM_APP, WM_CLOSE, WM_DESTROY, WM_ENDSESSION, WM_HOTKEY, WM_LBUTTONDBLCLK, WM_NCDESTROY,
    WM_POWERBROADCAST, WM_RBUTTONUP, WM_SETTINGCHANGE, WM_TIMER, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::PCWSTR;

use crate::capture_ui::CaptureSession;
use crate::color_pick_ui::ColorPickSession;
use crate::d2d::D2dRenderer;
use crate::history_ui::HistoryWindow;
use crate::hotkeys::HotKeyHost;
use crate::messages::{
    AppCommand, CaptureDoneMsg, ColorPickDoneMsg, NetSpeedPosMsg, OcrDoneMsg, ScrollPickMsg,
    TaskbarFxApplyMsg, UpdateCheckResult, WM_APP_CAPTURE_DONE, WM_APP_COLOR_PICK_DONE,
    WM_APP_COMMAND, WM_APP_HISTORY_CLOSED, WM_APP_NETSPEED_POS, WM_APP_NETSPEED_SAMPLE,
    WM_APP_OCR_DONE, WM_APP_PIN_CHANGED, WM_APP_PIN_MANAGE_CLOSED, WM_APP_SCROLL_DONE,
    WM_APP_SCROLL_PICKED, WM_APP_SCROLL_PROGRESS, WM_APP_SETTINGS_APPLY, WM_APP_SETTINGS_CLOSED,
    WM_APP_TASKBAR_FX_RESULT, WM_APP_TRAY_CALLBACK, WM_APP_UPDATE_CHECK_DONE, post_boxed,
    take_boxed,
};
use crate::netspeed_ui::{NetSpeedFloatWindow, NetSpeedTrayWidget};
use crate::pin_ui::{PinManageWindow, PinboardService};
use crate::scroll_ui::ScrollCaptureSession;
use crate::settings_ui::SettingsWindow;
use crate::tray::TrayIconService;
use crate::update_checker::UpdateChecker;
use crate::win32_util::{hinstance, register_window_class};

const CONTROLLER_CLASS: &str = "SuiteControllerWnd";
const TIMER_TRAY_WATCH: usize = 101;
const TIMER_SMOKE_EXIT: usize = 102;
const TIMER_SHUTDOWN_QUIT: usize = 103;
const WM_CONTEXTMENU_NOTIFY: u32 = 0x007B;
const WM_USER_NOTIFY_OPEN: u32 = 0x0402;
const PBT_APMRESUMESUSPEND: usize = 0x0007;
const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;
const WM_APP_SMOKE_DONE: u32 = WM_APP + 99;

#[derive(Debug, Clone, Serialize)]
pub struct SmokeReport {
    pub version: &'static str,
    pub pid: u32,
    pub working_set_bytes: usize,
    pub private_bytes: usize,
    pub monitors: usize,
    pub adapters: usize,
    pub ocr_available: bool,
    pub taskbar_kind: &'static str,
    pub samples_received: usize,
    pub clean_exit_pending: bool,
}

struct ControllerCore {
    hwnd: Cell<HWND>,
    smoke_check: bool,
    taskbar_created_msg: Cell<u32>,
    watcher_stop: Arc<AtomicBool>,
    shutting_down: Cell<bool>,

    store: RefCell<SettingsStore>,
    theme_service: ThemeToggleService<RegistryPersonalizeStore, NativeThemeBroadcaster>,
    sampler: Arc<NetSpeedSampler>,
    taskbar_fx: Arc<TaskbarFxEngine>,
    d2d: Rc<RefCell<D2dRenderer>>,

    tray: RefCell<TrayIconService>,
    hotkeys: RefCell<HotKeyHost>,
    netspeed_tray: NetSpeedTrayWidget,
    netspeed_secondary_trays: RefCell<Vec<NetSpeedTrayWidget>>,
    netspeed_float: NetSpeedFloatWindow,
    pins: RefCell<PinboardService>,

    capture_session: RefCell<Option<CaptureSession>>,
    color_pick_session: RefCell<Option<ColorPickSession>>,
    scroll_session: RefCell<Option<ScrollCaptureSession>>,

    settings_win: RefCell<Option<Rc<SettingsWindow>>>,
    history_win: RefCell<Option<Rc<HistoryWindow>>>,
    pin_manage_win: RefCell<Option<Rc<PinManageWindow>>>,

    last_start_label: RefCell<String>,
    last_capture_start: Cell<Option<Instant>>,
    hotkey_conflict_msg: RefCell<Option<String>>,
    last_fx_status_msg: RefCell<Option<String>>,

    taskbarfx_gen: Cell<u64>,
    taskbarfx_fail_count: Cell<i32>,
    taskbarfx_fail_window_start: Cell<Instant>,
    last_tray_hwnd: Cell<isize>,

    smoke_samples: Cell<usize>,
    smoke_report: RefCell<Option<SmokeReport>>,
}

pub struct AppController {
    core: Rc<ControllerCore>,
}

impl AppController {
    pub fn start(smoke_check: bool) -> Result<Self, String> {
        register_window_class(CONTROLLER_CLASS, Some(controller_wnd_proc), false, None);
        let cls = to_wide_null(CONTROLLER_CLASS);
        let title = to_wide_null("Suite Host");

        // Must be a hidden top-level WS_POPUP window (NOT HWND_MESSAGE) so we receive
        // RegisterWindowMessageW("TaskbarCreated"), WM_POWERBROADCAST, and WM_SETTINGCHANGE.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                PCWSTR(cls.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance()),
                None,
            )
            .map_err(|e| format!("Failed to create controller window: {e}"))?
        };

        let mut store = SettingsStore::new();
        let mut settings = store.load().clone();
        let degrade = CrashGuard::on_startup(&mut settings);
        if degrade.degraded {
            let _ = store.save(settings.clone());
        }
        SuiteLog::set_enabled(settings.advanced.logging_enabled);
        SuiteLog::info("Suite host starting");

        let d2d = Rc::new(RefCell::new(D2dRenderer::new()?));
        let sampler = Arc::new(NetSpeedSampler::new(
            Arc::new(IpHelperInterfaceTable),
            Duration::from_secs(1),
        ));
        sampler.set_preferred_adapter(settings.net_speed.if_index, settings.net_speed.adapter_alias.clone());

        let tray = TrayIconService::new(hwnd);
        let hotkeys = HotKeyHost::new(hwnd);
        let netspeed_tray = NetSpeedTrayWidget::new(hwnd, Rc::clone(&d2d), HWND::default());
        let netspeed_float = NetSpeedFloatWindow::new(hwnd, Rc::clone(&d2d));
        let pins = PinboardService::new(hwnd, Rc::clone(&d2d));

        let taskbar_created = unsafe { RegisterWindowMessageW(PCWSTR(to_wide_null("TaskbarCreated").as_ptr())) };
        let initial_tray = TaskbarProbe::find_primary_tray();

        let core = Rc::new(ControllerCore {
            hwnd: Cell::new(hwnd),
            smoke_check,
            taskbar_created_msg: Cell::new(taskbar_created),
            watcher_stop: Arc::new(AtomicBool::new(false)),
            shutting_down: Cell::new(false),
            store: RefCell::new(store),
            theme_service: ThemeToggleService::new_native(),
            sampler,
            taskbar_fx: Arc::new(TaskbarFxEngine::new()),
            d2d,
            tray: RefCell::new(tray),
            hotkeys: RefCell::new(hotkeys),
            netspeed_tray,
            netspeed_secondary_trays: RefCell::new(Vec::new()),
            netspeed_float,
            pins: RefCell::new(pins),
            capture_session: RefCell::new(None),
            color_pick_session: RefCell::new(None),
            scroll_session: RefCell::new(None),
            settings_win: RefCell::new(None),
            history_win: RefCell::new(None),
            pin_manage_win: RefCell::new(None),
            last_start_label: RefCell::new(degrade.last_start_label.clone()),
            last_capture_start: Cell::new(None),
            hotkey_conflict_msg: RefCell::new(None),
            last_fx_status_msg: RefCell::new(None),
            taskbarfx_gen: Cell::new(0),
            taskbarfx_fail_count: Cell::new(0),
            taskbarfx_fail_window_start: Cell::new(Instant::now()),
            last_tray_hwnd: Cell::new(initial_tray.0 as isize),
            smoke_samples: Cell::new(0),
            smoke_report: RefCell::new(None),
        });

        let raw_ptr = Rc::into_raw(Rc::clone(&core));
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw_ptr as isize);
        }

        let ctrl = Self { core };
        ctrl.initialize_runtime(&settings, degrade.message.as_deref());
        Ok(ctrl)
    }

    pub fn take_smoke_report(&self) -> Option<SmokeReport> {
        self.core.smoke_report.borrow_mut().take()
    }

    fn initialize_runtime(&self, settings: &AppSettings, degrade_msg: Option<&str>) {
        if !self.core.smoke_check
            && let Ok(exe) = std::env::current_exe()
            && let Err(e) = RunKeyService::try_set_enabled(settings.start_with_windows, &exe) {
                SuiteLog::error("RunKeyService", &e);
            }

        self.core.tray.borrow_mut().sync(settings);
        self.sync_theme();
        self.register_hotkeys(settings, false);

        if !self.core.smoke_check {
            self.apply_netspeed_presentation(settings, true);
        }

        if settings.net_speed.visible || self.core.smoke_check {
            let target_raw = self.core.hwnd.get().0 as isize;
            self.core.sampler.start(Arc::new(move |sample: RateSample| {
                post_boxed(HWND(target_raw as *mut _), WM_APP_NETSPEED_SAMPLE, 0, sample);
            }));
        }

        if !self.core.smoke_check {
            self.request_apply_taskbar_fx(settings.taskbar_fx.clone(), false);
            unsafe {
                let _ = SetTimer(Some(self.core.hwnd.get()), TIMER_TRAY_WATCH, 2000, None);
            }
            if let Some(msg) = degrade_msg {
                self.core.tray.borrow().balloon("Suite", msg);
            }
        } else {
            unsafe {
                let _ = SetTimer(Some(self.core.hwnd.get()), TIMER_SMOKE_EXIT, 2400, None);
            }
        }
    }

    fn register_hotkeys(&self, settings: &AppSettings, notify_on_conflict: bool) {
        let mut conflicts = Vec::new();
        let (cap_err, cap_code, pin_err, pin_code, alts) = {
            let mut hk = self.core.hotkeys.borrow_mut();
            let report = hk.register_all(
                &settings.hotkey,
                &settings.capture.color_pick_hotkey,
                &settings.capture.pin_click_through_hotkey,
                &settings.capture.scroll_capture_hotkey,
            );
            let cap_code = hk.last_capture_win32_error;
            let pin_code = hk.last_pin_win32_error;
            let alts: Vec<String> = hk
                .suggest_alternates(&settings.hotkey, 3)
                .into_iter()
                .map(|b| b.to_display_string())
                .collect();
            if report.color_pick_error.is_some() {
                conflicts.push(format!(
                    "取色热键 {} 注册失败。",
                    settings.capture.color_pick_hotkey.to_display_string()
                ));
            }
            if report.click_through_error.is_some() {
                conflicts.push(format!(
                    "穿透热键 {} 注册失败。",
                    settings.capture.pin_click_through_hotkey.to_display_string()
                ));
            }
            if report.scroll_capture_error.is_some() {
                conflicts.push(format!(
                    "滚动热键 {} 注册失败。",
                    settings.capture.scroll_capture_hotkey.to_display_string()
                ));
            }
            (report.capture_error, cap_code, report.pin_error, pin_code, alts)
        };

        if cap_err.is_some() {
            conflicts.insert(
                0,
                HotkeyCopy::capture_failed(&settings.hotkey.to_display_string(), cap_code, &alts),
            );
        }
        if pin_err.is_some() {
            conflicts.push(HotkeyCopy::pin_failed(pin_code, &alts));
        }

        if conflicts.is_empty() {
            *self.core.hotkey_conflict_msg.borrow_mut() = None;
            if let Some(ref sw) = *self.core.settings_win.borrow() {
                sw.clear_hotkey_conflict();
            }
        } else {
            let joined = conflicts.join(" ");
            SuiteLog::error("HotKeyHost", &joined);
            *self.core.hotkey_conflict_msg.borrow_mut() = Some(joined.clone());
            if let Some(ref sw) = *self.core.settings_win.borrow() {
                sw.set_hotkey_conflict(&joined);
            }
            if notify_on_conflict && !self.core.smoke_check {
                self.core.tray.borrow().balloon("Suite 热键冲突", &joined);
            }
        }
    }
}

impl Drop for AppController {
    fn drop(&mut self) {
        let hwnd = self.core.hwnd.get();
        if !hwnd.is_invalid() && !self.core.shutting_down.get() {
            shutdown_core(&self.core);
        }
    }
}

fn current_theme() -> ThemeKind {
    match PersonalizeRegistry::try_read() {
        Ok(d) if !d.system_uses_light => ThemeKind::Dark,
        _ => ThemeKind::Light,
    }
}

fn sync_theme_core(core: &ControllerCore, explicit: Option<ThemeKind>) {
    let theme = explicit.unwrap_or_else(current_theme);
    core.netspeed_tray.apply_theme(theme);
    for w in core.netspeed_secondary_trays.borrow().iter() {
        w.apply_theme(theme);
    }
    core.netspeed_float.apply_theme(theme);
    core.pins.borrow_mut().apply_theme(theme);
    if let Some(ref sw) = *core.settings_win.borrow() {
        sw.apply_theme(theme);
    }
}

impl AppController {
    fn sync_theme(&self) {
        sync_theme_core(&self.core, None);
    }

    fn apply_netspeed_presentation(&self, settings: &AppSettings, notify_on_fail: bool) {
        apply_netspeed_presentation_core(&self.core, settings, notify_on_fail);
    }

    fn request_apply_taskbar_fx(&self, fx: TaskbarFxSettings, shutdown_when_disabled: bool) {
        request_apply_taskbar_fx_core(&self.core, fx, shutdown_when_disabled);
    }
}

fn apply_netspeed_presentation_core(
    core: &ControllerCore,
    settings: &AppSettings,
    notify_on_fail: bool,
) {
    let theme = current_theme();
    core.netspeed_tray.apply_theme(theme);
    core.netspeed_tray.apply_appearance(&settings.net_speed);
    core.netspeed_float.apply_theme(theme);
    core.netspeed_float.apply_appearance(&settings.net_speed);

    if !settings.net_speed.visible {
        core.netspeed_tray.hide_embed();
        for w in core.netspeed_secondary_trays.borrow_mut().drain(..) {
            w.destroy();
        }
        core.netspeed_float.hide();
        return;
    }

    if settings.net_speed.embed_in_taskbar {
        match core.netspeed_tray.try_embed(None) {
            Ok(()) => {
                core.netspeed_float.hide();
                sync_secondary_trays_core(core, settings, theme);
            }
            Err(err) => {
                core.netspeed_tray.hide_embed();
                for w in core.netspeed_secondary_trays.borrow_mut().drain(..) {
                    w.destroy();
                }
                core.netspeed_float
                    .show_floating(settings.net_speed.left, settings.net_speed.top);
                let msg = NetSpeedCopy::embed_failed(Some(&err));
                SuiteLog::error("NetSpeedEmbed", &msg);
                if notify_on_fail {
                    core.tray.borrow().balloon("Suite 网速", &msg);
                }
                if let Some(ref sw) = *core.settings_win.borrow() {
                    sw.set_status(&msg);
                }
            }
        }
    } else {
        core.netspeed_tray.hide_embed();
        for w in core.netspeed_secondary_trays.borrow_mut().drain(..) {
            w.destroy();
        }
        core.netspeed_float
            .show_floating(settings.net_speed.left, settings.net_speed.top);
    }
}

fn sync_secondary_trays_core(core: &ControllerCore, settings: &AppSettings, theme: ThemeKind) {
    for w in core.netspeed_secondary_trays.borrow_mut().drain(..) {
        w.destroy();
    }
    if !settings.net_speed.embed_secondary {
        return;
    }
    let trays = TaskbarProbe::find_secondary_trays();
    let ctrl_hwnd = core.hwnd.get();
    let mut list = Vec::with_capacity(trays.len());
    for tray in trays {
        let w = NetSpeedTrayWidget::new(ctrl_hwnd, Rc::clone(&core.d2d), tray);
        w.apply_theme(theme);
        w.apply_appearance(&settings.net_speed);
        if w.try_embed(Some(tray)).is_ok() {
            list.push(w);
        } else {
            w.destroy();
        }
    }
    *core.netspeed_secondary_trays.borrow_mut() = list;
}

fn request_apply_taskbar_fx_core(
    core: &ControllerCore,
    fx: TaskbarFxSettings,
    shutdown_when_disabled: bool,
) {
    let gen_id = core.taskbarfx_gen.get().wrapping_add(1);
    core.taskbarfx_gen.set(gen_id);
    let was_enabled = fx.enabled;
    let engine = Arc::clone(&core.taskbar_fx);
    let hwnd_raw = core.hwnd.get().0 as isize;

    let _ = std::thread::Builder::new()
        .name("suite-taskbarfx".to_string())
        .spawn(move || {
            let status = engine.apply(&fx, shutdown_when_disabled);
            post_boxed(
                HWND(hwnd_raw as *mut _),
                WM_APP_TASKBAR_FX_RESULT,
                0,
                TaskbarFxApplyMsg {
                    gen_id,
                    status,
                    was_enabled,
                    shutdown_when_disabled,
                },
            );
        });
}

fn sync_pin_tray_and_manage_core(core: &ControllerCore) {
    let list = core.pins.borrow_mut().list();
    let any_through = list.iter().any(|p| p.is_click_through);
    core.tray.borrow_mut().set_click_through_available(any_through);
    if let Some(ref pm) = *core.pin_manage_win.borrow() {
        pm.reload(&list);
    }
}

fn sync_hotkeys_core(core: &ControllerCore, settings: &AppSettings, notify_on_conflict: bool) {
    let mut conflicts = Vec::new();
    let (cap_err, cap_code, pin_err, pin_code, alts) = {
        let mut hk = core.hotkeys.borrow_mut();
        let report = hk.register_all(
            &settings.hotkey,
            &settings.capture.color_pick_hotkey,
            &settings.capture.pin_click_through_hotkey,
            &settings.capture.scroll_capture_hotkey,
        );
        let cap_code = hk.last_capture_win32_error;
        let pin_code = hk.last_pin_win32_error;
        let alts: Vec<String> = hk
            .suggest_alternates(&settings.hotkey, 3)
            .into_iter()
            .map(|b| b.to_display_string())
            .collect();
        if report.color_pick_error.is_some() {
            conflicts.push(format!(
                "取色热键 {} 注册失败。",
                settings.capture.color_pick_hotkey.to_display_string()
            ));
        }
        if report.click_through_error.is_some() {
            conflicts.push(format!(
                "穿透热键 {} 注册失败。",
                settings.capture.pin_click_through_hotkey.to_display_string()
            ));
        }
        if report.scroll_capture_error.is_some() {
            conflicts.push(format!(
                "滚动热键 {} 注册失败。",
                settings.capture.scroll_capture_hotkey.to_display_string()
            ));
        }
        (report.capture_error, cap_code, report.pin_error, pin_code, alts)
    };

    if cap_err.is_some() {
        conflicts.insert(
            0,
            HotkeyCopy::capture_failed(&settings.hotkey.to_display_string(), cap_code, &alts),
        );
    }
    if pin_err.is_some() {
        conflicts.push(HotkeyCopy::pin_failed(pin_code, &alts));
    }

    if conflicts.is_empty() {
        *core.hotkey_conflict_msg.borrow_mut() = None;
        if let Some(ref sw) = *core.settings_win.borrow() {
            sw.clear_hotkey_conflict();
        }
    } else {
        let joined = conflicts.join(" ");
        *core.hotkey_conflict_msg.borrow_mut() = Some(joined.clone());
        if let Some(ref sw) = *core.settings_win.borrow() {
            sw.set_hotkey_conflict(&joined);
        }
        if notify_on_conflict {
            core.tray.borrow().balloon("Suite 热键冲突", &joined);
        }
    }
}

fn start_capture_core(core: &ControllerCore) {
    if core.capture_session.borrow().is_some()
        || core.color_pick_session.borrow().is_some()
        || core.scroll_session.borrow().is_some()
    {
        return;
    }
    if let Some(prev) = core.last_capture_start.get()
        && prev.elapsed() < Duration::from_millis(450)
    {
        return;
    }
    core.last_capture_start.set(Some(Instant::now()));

    let (pin_after, show_mag, save_dir) = {
        let st = core.store.borrow();
        let s = st.current();
        (
            s.capture.pin_after_capture,
            s.capture.show_magnifier,
            s.capture.save_directory.clone(),
        )
    };
    match CaptureSession::begin(
        core.hwnd.get(),
        Rc::clone(&core.d2d),
        pin_after,
        show_mag,
        save_dir,
    ) {
        Ok(sess) => {
            *core.capture_session.borrow_mut() = Some(sess);
        }
        Err(err) => {
            SuiteLog::error("CaptureSession", &err);
            core.tray.borrow().balloon("截图失败", &err);
        }
    }
}

fn start_color_pick_core(core: &ControllerCore) {
    if core.capture_session.borrow().is_some()
        || core.color_pick_session.borrow().is_some()
        || core.scroll_session.borrow().is_some()
    {
        return;
    }
    match ColorPickSession::begin(core.hwnd.get()) {
        Ok(sess) => {
            *core.color_pick_session.borrow_mut() = Some(sess);
        }
        Err(err) => {
            SuiteLog::error("ColorPick", &err);
            core.tray.borrow().balloon("取色", ColorPick::FAIL);
        }
    }
}

fn start_scroll_capture_core(core: &ControllerCore) {
    if core.capture_session.borrow().is_some()
        || core.color_pick_session.borrow().is_some()
        || core.scroll_session.borrow().is_some()
    {
        return;
    }
    match ScrollCaptureSession::begin_pick(core.hwnd.get()) {
        Ok(sess) => {
            *core.scroll_session.borrow_mut() = Some(sess);
        }
        Err(err) => {
            SuiteLog::error("ScrollCapture", &err);
            core.tray.borrow().balloon("滚动长截图", ScrollCapture::CAPTURE_FAILED);
        }
    }
}

fn open_settings_core(core: &ControllerCore) {
    let settings = core.store.borrow().current().clone();
    let adapters = core.sampler.list_adapters().unwrap_or_default();
    let last_label = core.last_start_label.borrow().clone();
    let existing = core.settings_win.borrow().clone();
    let win = SettingsWindow::show_or_focus(
        existing.as_deref(),
        core.hwnd.get(),
        &settings,
        &adapters,
        &last_label,
    );
    win.apply_theme(current_theme());
    if let Some(ref conflict) = *core.hotkey_conflict_msg.borrow() {
        win.set_hotkey_conflict(conflict);
    } else {
        win.clear_hotkey_conflict();
    }
    if let Some(ref fx_msg) = *core.last_fx_status_msg.borrow() {
        win.set_taskbar_status(fx_msg);
    }
    *core.settings_win.borrow_mut() = Some(win);
}

fn collect_smoke_report(core: &ControllerCore) -> SmokeReport {
    let mut mem = PROCESS_MEMORY_COUNTERS::default();
    mem.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    let (ws, priv_bytes) = unsafe {
        if GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut mem,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
        .is_ok()
        {
            (mem.WorkingSetSize, mem.PagefileUsage)
        } else {
            (0, 0)
        }
    };
    let monitors = MonitorEnumerator::get_all().len();
    let adapters = core.sampler.list_adapters().map(|v| v.len()).unwrap_or(0);
    let ocr_available = WindowsOcr::probe_available().is_some();
    let kind_str = match TaskbarProbe::detect() {
        TaskbarKind::ModernXaml => "ModernXaml",
        TaskbarKind::ClassicWin32 => "ClassicWin32",
        TaskbarKind::NotFound => "NotFound",
    };

    SmokeReport {
        version: UpdateChecker::LOCAL_VERSION,
        pid: std::process::id(),
        working_set_bytes: ws,
        private_bytes: priv_bytes,
        monitors,
        adapters,
        ocr_available,
        taskbar_kind: kind_str,
        samples_received: core.smoke_samples.get(),
        clean_exit_pending: true,
    }
}

fn shutdown_core(core: &ControllerCore) {
    if core.shutting_down.replace(true) {
        return;
    }
    core.watcher_stop.store(true, Ordering::Relaxed);
    let hwnd = core.hwnd.get();
    if !hwnd.is_invalid() {
        unsafe {
            let _ = KillTimer(Some(hwnd), TIMER_TRAY_WATCH);
            let _ = KillTimer(Some(hwnd), TIMER_SMOKE_EXIT);
        }
    }

    if let Some(sess) = core.capture_session.borrow_mut().take() {
        sess.cancel();
    }
    if let Some(sess) = core.color_pick_session.borrow_mut().take() {
        sess.cancel();
    }
    if let Some(mut sess) = core.scroll_session.borrow_mut().take() {
        sess.cancel();
    }

    if let Some(sw) = core.settings_win.borrow_mut().take() {
        sw.close();
    }
    if let Some(hw) = core.history_win.borrow_mut().take() {
        hw.close();
    }
    if let Some(pm) = core.pin_manage_win.borrow_mut().take() {
        pm.close();
    }
    core.pins.borrow_mut().close_all();

    core.hotkeys.borrow_mut().unregister_all();
    core.sampler.stop();

    core.netspeed_tray.destroy();
    for w in core.netspeed_secondary_trays.borrow_mut().drain(..) {
        w.destroy();
    }
    core.netspeed_float.hide();

    let fx = core.store.borrow().current().taskbar_fx.clone();
    if fx.enabled && !fx.keep_appearance_if_suite_exits && !core.smoke_check {
        let _ = core.taskbar_fx.reset();
    }

    if core.smoke_check && core.smoke_report.borrow().is_none() {
        *core.smoke_report.borrow_mut() = Some(collect_smoke_report(core));
    }

    CrashGuard::mark_clean_exit();
    SuiteLog::info("Suite host exiting cleanly");

    if !hwnd.is_invalid() {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}

fn handle_command(core: &Rc<ControllerCore>, cmd: AppCommand, lparam: LPARAM) {
    match cmd {
        AppCommand::ToggleTheme => {
            let res = core.theme_service.toggle_with_hook(|new_theme| {
                sync_theme_core(core, Some(new_theme));
            });
            if !res.succeeded {
                let err = res.error.unwrap_or_else(|| "无法切换深浅色主题。".to_string());
                SuiteLog::error("ThemeToggle", &err);
                core.tray.borrow().balloon("主题切换失败", &err);
            } else if !res.broadcast_succeeded {
                SuiteLog::error("ThemeBroadcast", "HWND_BROADCAST timed out");
            }
        }
        AppCommand::ToggleNetSpeed => {
            let updated = {
                let mut st = core.store.borrow_mut();
                let cur = st.current().clone();
                let mut next = cur.clone();
                next.net_speed.visible = !next.net_speed.visible;
                let _ = st.save(next.clone());
                next
            };
            core.tray.borrow_mut().sync(&updated);
            if updated.net_speed.visible {
                let target_raw = core.hwnd.get().0 as isize;
                core.sampler.start(Arc::new(move |sample: RateSample| {
                    post_boxed(HWND(target_raw as *mut _), WM_APP_NETSPEED_SAMPLE, 0, sample);
                }));
            } else {
                core.sampler.stop();
            }
            apply_netspeed_presentation_core(core, &updated, true);
            if let Some(ref sw) = *core.settings_win.borrow() {
                let adapters = core.sampler.list_adapters().unwrap_or_default();
                let lbl = core.last_start_label.borrow().clone();
                sw.reload(&updated, &adapters, &lbl);
            }
        }
        AppCommand::Capture => start_capture_core(core),
        AppCommand::ScrollCapture => start_scroll_capture_core(core),
        AppCommand::ColorPick => start_color_pick_core(core),
        AppCommand::History => {
            let settings = core.store.borrow().current().clone();
            let existing = core.history_win.borrow().clone();
            let win = HistoryWindow::show_or_focus(existing.as_deref(), core.hwnd.get(), &settings);
            *core.history_win.borrow_mut() = Some(win);
        }
        AppCommand::PinFromClipboard => {
            let res = core.pins.borrow_mut().try_pin_from_clipboard();
            match res {
                Ok(()) => sync_pin_tray_and_manage_core(core),
                Err(msg) => core.tray.borrow().balloon("贴图", &msg),
            }
        }
        AppCommand::PinManage => {
            let list = core.pins.borrow_mut().list();
            let existing = core.pin_manage_win.borrow().clone();
            let win = PinManageWindow::show_or_focus(existing.as_deref(), core.hwnd.get(), &list);
            *core.pin_manage_win.borrow_mut() = Some(win);
        }
        AppCommand::ClearClickThrough => {
            core.pins.borrow_mut().clear_click_through();
            sync_pin_tray_and_manage_core(core);
        }
        AppCommand::OpenSettings => open_settings_core(core),
        AppCommand::ToggleRunAtStartup => {
            let current = core.store.borrow().current().clone();
            let next_val = !current.start_with_windows;
            if let Ok(exe) = std::env::current_exe() {
                match RunKeyService::try_set_enabled(next_val, &exe) {
                    Ok(()) => {
                        let updated = {
                            let mut st = core.store.borrow_mut();
                            let mut next = current;
                            next.start_with_windows = next_val;
                            let _ = st.save(next.clone());
                            next
                        };
                        core.tray.borrow_mut().sync(&updated);
                        if let Some(ref sw) = *core.settings_win.borrow() {
                            let adapters = core.sampler.list_adapters().unwrap_or_default();
                            let lbl = core.last_start_label.borrow().clone();
                            sw.reload(&updated, &adapters, &lbl);
                        }
                    }
                    Err(err) => {
                        core.tray.borrow().balloon("开机启动设置失败", &err);
                    }
                }
            }
        }
        AppCommand::Exit => shutdown_core(core),
        AppCommand::RequestFloatNetSpeed => {
            let updated = {
                let mut st = core.store.borrow_mut();
                let mut next = st.current().clone();
                next.net_speed.embed_in_taskbar = false;
                let _ = st.save(next.clone());
                next
            };
            apply_netspeed_presentation_core(core, &updated, false);
            if let Some(ref sw) = *core.settings_win.borrow() {
                let adapters = core.sampler.list_adapters().unwrap_or_default();
                let lbl = core.last_start_label.borrow().clone();
                sw.reload(&updated, &adapters, &lbl);
            }
        }
        AppCommand::RequestEmbedNetSpeed => {
            let updated = {
                let mut st = core.store.borrow_mut();
                let mut next = st.current().clone();
                next.net_speed.embed_in_taskbar = true;
                next.net_speed.visible = true;
                let _ = st.save(next.clone());
                next
            };
            apply_netspeed_presentation_core(core, &updated, true);
            if let Some(ref sw) = *core.settings_win.borrow() {
                let adapters = core.sampler.list_adapters().unwrap_or_default();
                let lbl = core.last_start_label.borrow().clone();
                sw.reload(&updated, &adapters, &lbl);
            }
        }
        AppCommand::RefreshAdaptersAndSettings => {
            if let Some(ref sw) = *core.settings_win.borrow() {
                let settings = core.store.borrow().current().clone();
                let adapters = core.sampler.list_adapters().unwrap_or_default();
                let lbl = core.last_start_label.borrow().clone();
                sw.reload(&settings, &adapters, &lbl);
            }
        }
        AppCommand::CheckUpdates => {
            let update_cfg = core.store.borrow().current().update.clone();
            if let Some(ref sw) = *core.settings_win.borrow() {
                sw.set_status(UpdateChecker::CHECKING);
            }
            let hwnd_raw = core.hwnd.get().0 as isize;
            let _ = std::thread::Builder::new()
                .name("suite-update-check".to_string())
                .spawn(move || {
                    let res = UpdateChecker::check_blocking(&update_cfg);
                    post_boxed(HWND(hwnd_raw as *mut _), WM_APP_UPDATE_CHECK_DONE, 0, res);
                });
        }
        AppCommand::RestoreDefaults => {
            let defaults = AppSettings::create_default();
            if !core.smoke_check
                && let Ok(exe) = std::env::current_exe()
            {
                let _ = RunKeyService::try_set_enabled(defaults.start_with_windows, &exe);
            }
            let _ = core.store.borrow_mut().save(defaults.clone());
            apply_settings_change_core(core, defaults, false);
            if let Some(ref sw) = *core.settings_win.borrow() {
                let cur = core.store.borrow().current().clone();
                let adapters = core.sampler.list_adapters().unwrap_or_default();
                let lbl = core.last_start_label.borrow().clone();
                sw.reload(&cur, &adapters, &lbl);
                sw.set_status("已恢复默认设置。");
            }
        }
        AppCommand::TryNextHotkey => {
            let cur = core.store.borrow().current().clone();
            let next_opt = core.hotkeys.borrow().find_next_available_after(&cur.hotkey);
            match next_opt {
                Some(new_key) => {
                    let disp = new_key.to_display_string();
                    let mut next = cur;
                    next.hotkey = new_key;
                    let _ = core.store.borrow_mut().save(next.clone());
                    core.tray.borrow_mut().sync(&next);
                    sync_hotkeys_core(core, &next, false);
                    if let Some(ref sw) = *core.settings_win.borrow() {
                        let adapters = core.sampler.list_adapters().unwrap_or_default();
                        let lbl = core.last_start_label.borrow().clone();
                        sw.reload(&next, &adapters, &lbl);
                        sw.set_status(&HotkeyCopy::applied_alternate(&disp));
                    }
                }
                None => {
                    if let Some(ref sw) = *core.settings_win.borrow() {
                        sw.set_status(HotkeyCopy::NO_FREE_KEY);
                    } else {
                        core.tray.borrow().balloon("热键设置", HotkeyCopy::NO_FREE_KEY);
                    }
                }
            }
        }
        AppCommand::UseAcrylicReadable => {
            let mut next = core.store.borrow().current().clone();
            next.taskbar_fx.enabled = true;
            next.taskbar_fx.mode = TaskbarAppearanceModes::ACRYLIC.to_string();
            next.taskbar_fx.argb = TaskbarFxSettings::DEFAULT_ARGB;
            let _ = core.store.borrow_mut().save(next.clone());
            request_apply_taskbar_fx_core(core, next.taskbar_fx.clone(), false);
            if let Some(ref sw) = *core.settings_win.borrow() {
                let adapters = core.sampler.list_adapters().unwrap_or_default();
                let lbl = core.last_start_label.borrow().clone();
                sw.reload(&next, &adapters, &lbl);
            }
        }
        AppCommand::PinHistoryPath => {
            if let Some(path_str) = unsafe { take_boxed::<String>(lparam) } {
                match ScreenshotHistory::load_png(Path::new(&*path_str)) {
                    Ok(buf) => {
                        let dpi = MonitorEnumerator::dpi_near_cursor();
                        core.pins.borrow_mut().pin(buf, None, None, dpi, dpi);
                        sync_pin_tray_and_manage_core(core);
                    }
                    Err(err) => {
                        core.tray.borrow().balloon("贴图失败", &err);
                    }
                }
            }
        }
        AppCommand::FocusPin => {
            core.pins.borrow_mut().focus_by_index(lparam.0 as usize);
        }
        AppCommand::TogglePinThrough => {
            core.pins.borrow_mut().toggle_through_by_index(lparam.0 as usize);
            sync_pin_tray_and_manage_core(core);
        }
        AppCommand::ClosePin => {
            core.pins.borrow_mut().close_by_index(lparam.0 as usize);
            sync_pin_tray_and_manage_core(core);
        }
        AppCommand::CloseAllPins => {
            core.pins.borrow_mut().close_all();
            sync_pin_tray_and_manage_core(core);
        }
    }
}

fn apply_settings_change_core(core: &ControllerCore, mut next: AppSettings, notify: bool) {
    next.capture.normalize();
    next.net_speed.normalize();
    next.update.normalize();
    let prev = core.store.borrow().current().clone();

    if !core.smoke_check
        && prev.start_with_windows != next.start_with_windows
        && let Ok(exe) = std::env::current_exe()
        && let Err(err) = RunKeyService::try_set_enabled(next.start_with_windows, &exe) {
            next.start_with_windows = prev.start_with_windows;
            if let Some(ref sw) = *core.settings_win.borrow() {
                sw.set_status(&err);
            }
        }

    let _ = core.store.borrow_mut().save(next.clone());
    SuiteLog::set_enabled(next.advanced.logging_enabled);
    core.tray.borrow_mut().sync(&next);

    if prev.hotkey != next.hotkey
        || prev.capture.color_pick_hotkey != next.capture.color_pick_hotkey
        || prev.capture.pin_click_through_hotkey != next.capture.pin_click_through_hotkey
        || prev.capture.scroll_capture_hotkey != next.capture.scroll_capture_hotkey
    {
        sync_hotkeys_core(core, &next, notify);
    }

    if prev.net_speed.if_index != next.net_speed.if_index
        || prev.net_speed.adapter_alias != next.net_speed.adapter_alias
    {
        core.sampler
            .set_preferred_adapter(next.net_speed.if_index, next.net_speed.adapter_alias.clone());
    }

    if prev.net_speed.visible != next.net_speed.visible {
        if next.net_speed.visible {
            let target_raw = core.hwnd.get().0 as isize;
            core.sampler.start(Arc::new(move |sample: RateSample| {
                post_boxed(HWND(target_raw as *mut _), WM_APP_NETSPEED_SAMPLE, 0, sample);
            }));
        } else if !core.smoke_check {
            core.sampler.stop();
        }
    }

    if prev.net_speed != next.net_speed && !core.smoke_check {
        apply_netspeed_presentation_core(core, &next, notify);
    }

    if prev.taskbar_fx != next.taskbar_fx && !core.smoke_check {
        core.taskbarfx_fail_count.set(0);
        request_apply_taskbar_fx_core(core, next.taskbar_fx.clone(), false);
    }
}

unsafe extern "system" fn controller_wnd_proc(
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
                drop(Rc::from_raw(raw as *const ControllerCore));
            }
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let core = unsafe {
        let r = Rc::from_raw(ptr as *const ControllerCore);
        let c = Rc::clone(&r);
        std::mem::forget(r);
        c
    };

    if msg == core.taskbar_created_msg.get() && core.taskbar_created_msg.get() != 0 {
        SuiteLog::info("Explorer TaskbarCreated received; restoring tray + embed + TaskbarFx");
        core.tray.borrow_mut().add_or_restore();
        let settings = core.store.borrow().current().clone();
        core.tray.borrow_mut().sync(&settings);
        if !core.smoke_check {
            apply_netspeed_presentation_core(&core, &settings, false);
            if settings.taskbar_fx.enabled {
                core.taskbarfx_fail_count.set(0);
                request_apply_taskbar_fx_core(&core, settings.taskbar_fx.clone(), false);
            }
        }
        return LRESULT(0);
    }

    match msg {
        WM_APP_TRAY_CALLBACK => {
            let notif = (lparam.0 & 0xFFFF) as u32;
            match notif {
                WM_RBUTTONUP | WM_CONTEXTMENU_NOTIFY => {
                    let args = core.tray.borrow().menu_args();
                    TrayIconService::show_popup_menu(&args);
                }
                WM_LBUTTONDBLCLK | WM_USER_NOTIFY_OPEN => {
                    open_settings_core(&core);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            let id = wparam.0 as i32;
            if id == suite_platform::constants::HOTKEY_ID_CAPTURE {
                start_capture_core(&core);
            } else if id == suite_platform::constants::HOTKEY_ID_PIN_CLIPBOARD {
                if let Err(err) = core.pins.borrow_mut().try_pin_from_clipboard() {
                    core.tray.borrow().balloon("贴图", &err);
                } else {
                    sync_pin_tray_and_manage_core(&core);
                }
            } else if id == suite_platform::constants::HOTKEY_ID_COLOR_PICK {
                start_color_pick_core(&core);
            } else if id == suite_platform::constants::HOTKEY_ID_PIN_CLICK_THROUGH {
                match core.pins.borrow_mut().try_toggle_top_click_through() {
                    Ok(()) => sync_pin_tray_and_manage_core(&core),
                    Err(err) => core.tray.borrow().balloon("贴图穿透", &err),
                }
            } else if id == suite_platform::constants::HOTKEY_ID_SCROLL_CAPTURE {
                start_scroll_capture_core(&core);
            }
            LRESULT(0)
        }
        WM_APP_COMMAND => {
            if let Some(cmd) = AppCommand::from_wparam(wparam) {
                handle_command(&core, cmd, lparam);
            }
            LRESULT(0)
        }
        WM_APP_NETSPEED_SAMPLE => {
            if let Some(sample) = unsafe { take_boxed::<RateSample>(lparam) } {
                core.smoke_samples.set(core.smoke_samples.get().saturating_add(1));
                core.netspeed_tray.apply_sample(&sample);
                for w in core.netspeed_secondary_trays.borrow().iter() {
                    w.apply_sample(&sample);
                }
                core.netspeed_float.apply_sample(&sample);
            }
            LRESULT(0)
        }
        WM_APP_NETSPEED_POS => {
            if let Some(pos) = unsafe { take_boxed::<NetSpeedPosMsg>(lparam) } {
                let mut st = core.store.borrow_mut();
                let _ = st.update(|s| {
                    s.net_speed.left = pos.left;
                    s.net_speed.top = pos.top;
                });
            }
            LRESULT(0)
        }
        WM_APP_TASKBAR_FX_RESULT => {
            if let Some(msg) = unsafe { take_boxed::<TaskbarFxApplyMsg>(lparam) } {
                if msg.gen_id < core.taskbarfx_gen.get() {
                    return LRESULT(0);
                }
                let text = msg.status.message.clone();
                *core.last_fx_status_msg.borrow_mut() = Some(text.clone());
                if let Some(ref sw) = *core.settings_win.borrow() {
                    sw.set_taskbar_status(&text);
                }
                if msg.status.result_code != 0 && msg.was_enabled && !msg.shutdown_when_disabled {
                    SuiteLog::error("TaskbarFx", &text);
                    if core.taskbarfx_fail_window_start.get().elapsed() > Duration::from_secs(30) {
                        core.taskbarfx_fail_count.set(0);
                        core.taskbarfx_fail_window_start.set(Instant::now());
                    }
                    let count = core.taskbarfx_fail_count.get() + 1;
                    core.taskbarfx_fail_count.set(count);
                    let auto_disable = core
                        .store
                        .borrow()
                        .current()
                        .advanced
                        .taskbar_fx_auto_disable_on_fail;
                    if auto_disable && count >= 5 {
                        let updated = {
                            let mut st = core.store.borrow_mut();
                            let mut next = st.current().clone();
                            next.taskbar_fx.enabled = false;
                            let _ = st.save(next.clone());
                            next
                        };
                        let off_msg = format!("{}（连续失败过多，已自动关闭以防闪烁）", TaskbarFxCopy::CLOSED);
                        *core.last_fx_status_msg.borrow_mut() = Some(off_msg.clone());
                        core.tray.borrow().balloon("任务栏效果", &off_msg);
                        if let Some(ref sw) = *core.settings_win.borrow() {
                            let adapters = core.sampler.list_adapters().unwrap_or_default();
                            let lbl = core.last_start_label.borrow().clone();
                            sw.reload(&updated, &adapters, &lbl);
                            sw.set_taskbar_status(&off_msg);
                        }
                    } else if count == 1 {
                        core.tray.borrow().balloon("任务栏效果", &text);
                    }
                } else {
                    core.taskbarfx_fail_count.set(0);
                }
            }
            LRESULT(0)
        }
        WM_APP_OCR_DONE => {
            if let Some(msg) = unsafe { take_boxed::<OcrDoneMsg>(lparam) }
                && let Some(ref sess) = *core.capture_session.borrow() {
                    sess.handle_ocr_done(*msg);
                }
            LRESULT(0)
        }
        WM_APP_CAPTURE_DONE => {
            *core.capture_session.borrow_mut() = None;
            if let Some(done) = unsafe { take_boxed::<CaptureDoneMsg>(lparam) } {
                if let Some(err) = done.error {
                    SuiteLog::error("CaptureDone", &err);
                    core.tray.borrow().balloon("截图", &err);
                } else if !done.cancelled
                    && let Some(mut img) = done.image
                {
                    let settings = core.store.borrow().current().clone();
                    let _ = ScreenshotHistory::try_add(&img, &settings);
                    if let Some(ref hw) = *core.history_win.borrow() {
                        hw.reload(&settings);
                    }
                    if done.save_requested || settings.capture.save_file_after_capture {
                        let dir_opt = (!settings.capture.save_directory.trim().is_empty())
                            .then_some(settings.capture.save_directory.as_str());
                        if let Err(err) = save_png_atomic(&img, dir_opt) {
                            SuiteLog::error("SaveCapturePng", &err);
                            core.tray.borrow().balloon("保存截图失败", &err);
                        }
                    }
                    if done.pin_requested || settings.capture.pin_after_capture {
                        let left_dip = Some(done.selection.x as f64 * 96.0 / done.dpi_x.max(96) as f64);
                        let top_dip = Some(done.selection.y as f64 * 96.0 / done.dpi_y.max(96) as f64);
                        core.pins
                            .borrow_mut()
                            .pin(img, left_dip, top_dip, done.dpi_x, done.dpi_y);
                        sync_pin_tray_and_manage_core(&core);
                    } else {
                        img.release_pixels();
                    }
                }
            }
            LRESULT(0)
        }
        WM_APP_COLOR_PICK_DONE => {
            *core.color_pick_session.borrow_mut() = None;
            if let Some(done) = unsafe { take_boxed::<ColorPickDoneMsg>(lparam) } {
                if let Some(err) = done.error {
                    core.tray.borrow().balloon("取色", &err);
                } else if let Some(val) = done.copied_value {
                    core.tray.borrow().balloon("取色", &ColorPick::copied_message(&val));
                }
            }
            LRESULT(0)
        }
        WM_APP_SCROLL_PICKED => {
            if let Some(pick) = unsafe { take_boxed::<ScrollPickMsg>(lparam) } {
                if let Some(rect) = pick.rect {
                    if let Some(ref mut sess) = *core.scroll_session.borrow_mut() {
                        sess.start_scrolling(core.hwnd.get(), &core.d2d, rect);
                    }
                } else {
                    *core.scroll_session.borrow_mut() = None;
                }
            }
            LRESULT(0)
        }
        WM_APP_SCROLL_PROGRESS => LRESULT(0),
        WM_APP_SCROLL_DONE => {
            *core.scroll_session.borrow_mut() = None;
            if let Some(res) = unsafe { take_boxed::<suite_capture::ScrollCaptureResult>(lparam) } {
                if res.succeeded
                    && let Some(mut img) = res.image
                {
                    let settings = core.store.borrow().current().clone();
                    let _ = ScreenshotHistory::try_add(&img, &settings);
                    if let Some(ref hw) = *core.history_win.borrow() {
                        hw.reload(&settings);
                    }
                    let copy_ok = ImageClipboard::copy(&img).is_ok();
                    if settings.capture.save_file_after_capture {
                        let dir_opt = (!settings.capture.save_directory.trim().is_empty())
                            .then_some(settings.capture.save_directory.as_str());
                        if save_png_atomic(&img, dir_opt).is_ok() {
                            core.tray
                                .borrow()
                                .balloon("滚动长截图", ScrollCapture::COPIED_AND_SAVED);
                        } else if copy_ok {
                            core.tray.borrow().balloon("滚动长截图", ScrollCapture::COPIED);
                        }
                    } else if copy_ok {
                        core.tray.borrow().balloon("滚动长截图", ScrollCapture::COPIED);
                    }
                    img.release_pixels();
                } else if let Some(err) = res.error {
                    core.tray.borrow().balloon("滚动长截图", &err);
                }
            }
            LRESULT(0)
        }
        WM_APP_UPDATE_CHECK_DONE => {
            if let Some(res) = unsafe { take_boxed::<UpdateCheckResult>(lparam) } {
                SuiteLog::info(&format!(
                    "UpdateCheck ok={} channel={:?} version={:?} url={:?}",
                    res.ok, res.channel_zh, res.version, res.html_url
                ));
                if let Some(ref sw) = *core.settings_win.borrow() {
                    sw.set_status(&res.message);
                } else if res.has_update {
                    core.tray.borrow().balloon("Suite 发现新版本", &res.message);
                }
            }
            LRESULT(0)
        }
        WM_APP_SETTINGS_APPLY => {
            if let Some(next) = unsafe { take_boxed::<AppSettings>(lparam) } {
                apply_settings_change_core(&core, *next, true);
            }
            LRESULT(0)
        }
        WM_APP_SETTINGS_CLOSED => {
            *core.settings_win.borrow_mut() = None;
            LRESULT(0)
        }
        WM_APP_HISTORY_CLOSED => {
            *core.history_win.borrow_mut() = None;
            LRESULT(0)
        }
        WM_APP_PIN_CHANGED => {
            sync_pin_tray_and_manage_core(&core);
            LRESULT(0)
        }
        WM_APP_PIN_MANAGE_CLOSED => {
            *core.pin_manage_win.borrow_mut() = None;
            LRESULT(0)
        }
        WM_SETTINGCHANGE => {
            sync_theme_core(&core, None);
            LRESULT(0)
        }
        WM_POWERBROADCAST => {
            if wparam.0 == PBT_APMRESUMESUSPEND || wparam.0 == PBT_APMRESUMEAUTOMATIC {
                SuiteLog::info("Power resume detected; resetting net speed baseline");
                core.sampler.reset_after_resume();
            }
            LRESULT(1)
        }
        WM_TIMER => {
            let tid = wparam.0;
            if tid == TIMER_TRAY_WATCH && !core.shutting_down.get() {
                let tray = TaskbarProbe::find_primary_tray();
                let tray_raw = tray.0 as isize;
                let prev_raw = core.last_tray_hwnd.get();
                if tray_raw != 0 && tray_raw != prev_raw {
                    core.last_tray_hwnd.set(tray_raw);
                    let settings = core.store.borrow().current().clone();
                    if settings.taskbar_fx.enabled && !core.smoke_check {
                        SuiteLog::info("Primary tray HWND changed; re-applying TaskbarFx");
                        core.taskbarfx_fail_count.set(0);
                        request_apply_taskbar_fx_core(&core, settings.taskbar_fx, false);
                    }
                }
            } else if tid == TIMER_SMOKE_EXIT && core.smoke_check {
                unsafe {
                    let _ = KillTimer(Some(hwnd), TIMER_SMOKE_EXIT);
                }
                *core.smoke_report.borrow_mut() = Some(collect_smoke_report(&core));
                shutdown_core(&core);
            } else if tid == TIMER_SHUTDOWN_QUIT {
                unsafe {
                    let _ = PostMessageW(Some(hwnd), WM_APP_SMOKE_DONE, WPARAM(0), LPARAM(0));
                }
            }
            LRESULT(0)
        }
        WM_CLOSE | WM_ENDSESSION => {
            shutdown_core(&core);
            LRESULT(0)
        }
        WM_DESTROY => {
            core.hwnd.set(HWND::default());
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

#[allow(dead_code)]
fn _keep_types_linked(_: PixelRect) {}
