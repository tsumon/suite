use core::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use suite_contracts::TaskbarAppearanceMode;
use suite_platform::TaskbarProbe;
use suite_platform::constants::{TASKBAR_FX_TAP_FILE_NAME, to_wide_null};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE, HWND, LPARAM, WAIT_OBJECT_0, WPARAM};
use windows::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetProcAddress, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile,
    PAGE_READWRITE, UnmapViewOfFile,
};
use windows::Win32::System::Threading::{
    CreateEventW, GetCurrentProcessId, ResetEvent, SetEvent, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowThreadProcessId, HHOOK, HOOKPROC, SMTO_ABORTIFHUNG, SendMessageTimeoutW,
    SetWindowsHookExW, UnhookWindowsHookEx, WH_CALLWNDPROC, WM_NULL,
};
use windows::core::{GUID, HRESULT, PCSTR, PCWSTR};

use crate::protocol::{
    CLSID_TASKBAR_FX_TAP_SITE, K_TASKBAR_FX_TAP_APPLY_EVENT_NAME, K_TASKBAR_FX_TAP_MAGIC,
    K_TASKBAR_FX_TAP_READY_EVENT_NAME, K_TASKBAR_FX_TAP_SHM_NAME, K_TASKBAR_FX_TAP_VERSION,
    TapResultCode, TaskbarFxResultCode, TaskbarFxTapShm,
};

type InitializeXamlDiagnosticsExFn = unsafe extern "system" fn(
    PCWSTR,
    u32,
    PCWSTR,
    PCWSTR,
    GUID,
    PCWSTR,
) -> HRESULT;

pub struct Win11TapClient {
    mapping: HANDLE,
    shm: *mut TaskbarFxTapShm,
    ready_event: HANDLE,
    apply_event: HANDLE,
    tap_module: HMODULE,
    injected: bool,
    explorer_pid: u32,
}

unsafe impl Send for Win11TapClient {}

impl Win11TapClient {
    pub fn new() -> Self {
        Self {
            mapping: HANDLE::default(),
            shm: std::ptr::null_mut(),
            ready_event: HANDLE::default(),
            apply_event: HANDLE::default(),
            tap_module: HMODULE::default(),
            injected: false,
            explorer_pid: 0,
        }
    }

    fn tap_dll_path() -> Option<PathBuf> {
        let mut buf = vec![0u16; 1024];
        let len = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
        if len == 0 {
            return None;
        }
        let exe = PathBuf::from(String::from_utf16_lossy(&buf[..len]));
        let dir = exe.parent()?;
        Some(dir.join(TASKBAR_FX_TAP_FILE_NAME))
    }

    fn explorer_pid_and_tid_from_tray() -> (u32, u32) {
        let tray: HWND = TaskbarProbe::find_primary_tray();
        if tray.is_invalid() {
            return (0, 0);
        }
        let mut pid = 0u32;
        let tid = unsafe { GetWindowThreadProcessId(tray, Some(&mut pid)) };
        (pid, tid)
    }

    fn ensure_shm(&mut self) -> Result<(), String> {
        if !self.shm.is_null() {
            return Ok(());
        }
        unsafe {
            let ready_name = to_wide_null(K_TASKBAR_FX_TAP_READY_EVENT_NAME);
            let apply_name = to_wide_null(K_TASKBAR_FX_TAP_APPLY_EVENT_NAME);
            let shm_name = to_wide_null(K_TASKBAR_FX_TAP_SHM_NAME);

            self.ready_event = CreateEventW(None, true, false, PCWSTR(ready_name.as_ptr()))
                .map_err(|e| format!("CreateEventW(Ready) failed: {e}"))?;
            self.apply_event = CreateEventW(None, false, false, PCWSTR(apply_name.as_ptr()))
                .map_err(|e| format!("CreateEventW(Apply) failed: {e}"))?;
            self.mapping = CreateFileMappingW(
                windows::Win32::Foundation::INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                std::mem::size_of::<TaskbarFxTapShm>() as u32,
                PCWSTR(shm_name.as_ptr()),
            )
            .map_err(|e| format!("CreateFileMappingW failed: {e}"))?;

            let view = MapViewOfFile(
                self.mapping,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                std::mem::size_of::<TaskbarFxTapShm>(),
            );
            if view.Value.is_null() {
                return Err("MapViewOfFile returned null.".to_string());
            }
            self.shm = view.Value.cast::<TaskbarFxTapShm>();
            let shm_ref = &mut *self.shm;
            if shm_ref.magic != K_TASKBAR_FX_TAP_MAGIC {
                std::ptr::write_bytes(self.shm, 0, 1);
                shm_ref.magic = K_TASKBAR_FX_TAP_MAGIC;
                shm_ref.version = K_TASKBAR_FX_TAP_VERSION;
            }
            shm_ref
                .host_pid
                .store(GetCurrentProcessId(), Ordering::Release);
        }
        Ok(())
    }

    fn wait_ready(&self, timeout_ms: u32) -> bool {
        if self.ready_event.is_invalid() {
            return false;
        }
        unsafe { WaitForSingleObject(self.ready_event, timeout_ms) == WAIT_OBJECT_0 }
    }

    fn ping_tap_pump(&self, timeout: Duration) -> bool {
        if self.shm.is_null() || self.apply_event.is_invalid() {
            return false;
        }
        let shm = unsafe { &*self.shm };
        let next = shm.seq.load(Ordering::Acquire).wrapping_add(1);
        shm.mode.store(TaskbarAppearanceMode::Normal as i32, Ordering::Relaxed);
        shm.argb.store(0, Ordering::Relaxed);
        shm.result.store(TapResultCode::Failed as i32, Ordering::Relaxed);
        shm.seq.store(next, Ordering::Release);
        unsafe {
            let _ = SetEvent(self.apply_event);
        }
        let start = Instant::now();
        while start.elapsed() < timeout {
            if shm.ack.load(Ordering::Acquire) == next {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    fn hook_fallback(&mut self, tap_path: &Path, tid: u32) -> Result<bool, String> {
        let wide_path = to_wide_null(&tap_path.to_string_lossy());
        if self.tap_module.is_invalid() {
            let loaded = unsafe {
                LoadLibraryExW(
                    PCWSTR(wide_path.as_ptr()),
                    None,
                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
                )
            };
            self.tap_module = match loaded {
                Ok(m) => m,
                Err(e) => {
                    let msg = format!("{e}");
                    if msg.contains("拒绝") || msg.to_ascii_lowercase().contains("denied") {
                        return Err("安全软件拦截了 TaskbarFx.Tap.dll。".to_string());
                    }
                    return Err("TaskbarFx.Tap.dll could not be loaded.".to_string());
                }
            };
        }

        let proc = unsafe {
            GetProcAddress(
                self.tap_module,
                PCSTR(c"TaskbarFxTapCallWndProc".as_ptr().cast()),
            )
        };
        let Some(proc_fn) = proc else {
            return Err("TaskbarFx.Tap.dll is missing TaskbarFxTapCallWndProc.".to_string());
        };
        let hook_proc: HOOKPROC = Some(unsafe {
            std::mem::transmute::<unsafe extern "system" fn() -> isize, unsafe extern "system" fn(i32, WPARAM, LPARAM) -> windows::Win32::Foundation::LRESULT>(proc_fn)
        });

        let hook: HHOOK =
            unsafe { SetWindowsHookExW(WH_CALLWNDPROC, hook_proc, Some(self.tap_module.into()), tid) }
            .map_err(|_| "Could not load the Win11 taskbar helper into Explorer.".to_string())?;

        let tray = TaskbarProbe::find_primary_tray();
        if !tray.is_invalid() {
            unsafe {
                let _ = SendMessageTimeoutW(
                    tray,
                    WM_NULL,
                    WPARAM(0),
                    LPARAM(0),
                    SMTO_ABORTIFHUNG,
                    1000,
                    None,
                );
            }
        }

        let ready = self.wait_ready(35_000);
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
        Ok(ready)
    }

    fn call_ixde(tap_path: &Path, pid: u32) -> bool {
        let wux_name = to_wide_null("Windows.UI.Xaml.dll");
        let Ok(wux) = (unsafe { LoadLibraryExW(PCWSTR(wux_name.as_ptr()), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }) else {
            return false;
        };
        let Some(proc) = (unsafe {
            GetProcAddress(wux, PCSTR(c"InitializeXamlDiagnosticsEx".as_ptr().cast()))
        }) else {
            return false;
        };
        let ixde: InitializeXamlDiagnosticsExFn = unsafe {
            std::mem::transmute::<unsafe extern "system" fn() -> isize, InitializeXamlDiagnosticsExFn>(
                proc,
            )
        };
        let tap_wide = to_wide_null(&tap_path.to_string_lossy());
        for attempt in 1..=5 {
            let conn = to_wide_null(&format!("SuiteTaskbarFxDiag{attempt}"));
            let hr = unsafe {
                ixde(
                    PCWSTR(conn.as_ptr()),
                    pid,
                    PCWSTR::null(),
                    PCWSTR(tap_wide.as_ptr()),
                    CLSID_TASKBAR_FX_TAP_SITE,
                    PCWSTR::null(),
                )
            };
            if hr.is_ok() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        false
    }

    fn ensure_injected(&mut self) -> Result<(), (TaskbarFxResultCode, String)> {
        let (pid, tid) = Self::explorer_pid_and_tid_from_tray();
        if pid == 0 || tid == 0 {
            return Err((
                TaskbarFxResultCode::ErrNoTray,
                "No Shell_TrayWnd. Taskbar is not available.".to_string(),
            ));
        }
        self.ensure_shm().map_err(|_| {
            (
                TaskbarFxResultCode::ErrTapInit,
                "Could not create the Win11 taskbar helper channel.".to_string(),
            )
        })?;

        let shm = unsafe { &*self.shm };
        if self.injected && self.explorer_pid == pid && self.ping_tap_pump(Duration::from_millis(400)) {
            return Ok(());
        }
        if shm.explorer_pid.load(Ordering::Acquire) == pid
            && self.ping_tap_pump(Duration::from_millis(400))
        {
            self.injected = true;
            self.explorer_pid = pid;
            shm.ready.store(1, Ordering::Release);
            return Ok(());
        }

        self.injected = false;
        self.explorer_pid = 0;
        shm.ready.store(0, Ordering::Release);

        let Some(tap_path) = Self::tap_dll_path().filter(|p| p.is_file()) else {
            return Err((
                TaskbarFxResultCode::ErrTapInit,
                "TaskbarFx.Tap.dll missing beside Suite.exe.".to_string(),
            ));
        };

        unsafe {
            let _ = ResetEvent(self.ready_event);
        }

        let hook_ready = match self.hook_fallback(&tap_path, tid) {
            Ok(ready) => ready,
            Err(msg) => {
                if msg.contains("安全软件") {
                    return Err((TaskbarFxResultCode::ErrTapInit, msg));
                }
                false
            }
        };

        if !hook_ready {
            let ixde_ok = Self::call_ixde(&tap_path, pid);
            if !self.wait_ready(if ixde_ok { 35_000 } else { 2_000 }) {
                return Err((
                    TaskbarFxResultCode::ErrTapInit,
                    "安全软件可能拦截了 Win11 任务栏辅助模块。".to_string(),
                ));
            }
        }

        self.injected = true;
        self.explorer_pid = pid;
        shm.ready.store(1, Ordering::Release);
        shm.explorer_pid.store(pid, Ordering::Release);

        if !self.ping_tap_pump(Duration::from_millis(1000)) {
            return Err((
                TaskbarFxResultCode::ErrTapInit,
                "Win11 taskbar helper started but did not respond.".to_string(),
            ));
        }

        Ok(())
    }

    pub fn apply(&mut self, mode: TaskbarAppearanceMode, argb: u32) -> (TaskbarFxResultCode, String) {
        if let Err((code, msg)) = self.ensure_injected() {
            return (code, msg);
        }

        let shm = unsafe { &*self.shm };
        let next = shm.seq.load(Ordering::Acquire).wrapping_add(1);
        shm.mode.store(mode as i32, Ordering::Release);
        shm.argb.store(argb, Ordering::Release);
        shm.result.store(TapResultCode::PendingFills as i32, Ordering::Release);
        shm.fill_count.store(0, Ordering::Release);
        shm.seq.store(next, Ordering::Release);
        unsafe {
            let _ = SetEvent(self.apply_event);
        }

        let start = Instant::now();
        let pending_wait = Duration::from_millis(8000);
        while start.elapsed() < pending_wait {
            let ack = shm.ack.load(Ordering::Acquire);
            let res = shm.result.load(Ordering::Acquire);
            if ack == next && res != TapResultCode::PendingFills as i32 {
                break;
            }
            if ack == next && res == TapResultCode::PendingFills as i32 {
                unsafe {
                    let _ = SetEvent(self.apply_event);
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        let ack = shm.ack.load(Ordering::Acquire);
        let res = shm.result.load(Ordering::Acquire);
        if ack != next || res == TapResultCode::PendingFills as i32 {
            return (
                TaskbarFxResultCode::ErrCallFailed,
                "暂时找不到任务栏背景，请再点一次应用。".to_string(),
            );
        }
        if res == TapResultCode::Failed as i32 {
            return (
                TaskbarFxResultCode::ErrCallFailed,
                "Win11 taskbar appearance could not be applied.".to_string(),
            );
        }
        if mode == TaskbarAppearanceMode::Normal {
            (
                TaskbarFxResultCode::Ok,
                "Restored system default (did not end explorer).".to_string(),
            )
        } else if res == TapResultCode::AcrylicFallback as i32 {
            (
                TaskbarFxResultCode::Ok,
                "Applied Win11 XAML appearance. Acrylic fell back to a solid tint.".to_string(),
            )
        } else {
            (
                TaskbarFxResultCode::Ok,
                "Applied Win11 XAML appearance.".to_string(),
            )
        }
    }
}

impl Default for Win11TapClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Win11TapClient {
    fn drop(&mut self) {
        unsafe {
            if !self.shm.is_null() {
                let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.shm.cast::<c_void>(),
                });
                self.shm = std::ptr::null_mut();
            }
            if !self.mapping.is_invalid() {
                let _ = CloseHandle(self.mapping);
            }
            if !self.ready_event.is_invalid() {
                let _ = CloseHandle(self.ready_event);
            }
            if !self.apply_event.is_invalid() {
                let _ = CloseHandle(self.apply_event);
            }
            // Intentionally do NOT FreeLibrary(self.tap_module) if hook was used, or keep handle minimal.
        }
    }
}
