use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use chrono::Local;
use suite_contracts::SettingsPaths;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::PCWSTR;

use crate::constants::to_wide_null;

static ENABLED: AtomicBool = AtomicBool::new(false);
static LOG_LOCK: Mutex<()> = Mutex::new(());

pub struct SuiteLog;

impl SuiteLog {
    pub fn set_enabled(enabled: bool) {
        ENABLED.store(enabled, Ordering::Relaxed);
    }

    pub fn is_enabled() -> bool {
        ENABLED.load(Ordering::Relaxed)
    }

    pub fn info(message: &str) {
        if !Self::is_enabled() {
            return;
        }
        Self::write_line("INFO", message);
    }

    pub fn error(context: &str, err: &str) {
        if !Self::is_enabled() {
            return;
        }
        Self::write_line("ERROR", &format!("{context}: {err}"));
    }

    pub fn try_write_probe() -> Result<(), String> {
        let dir = SettingsPaths::logs_directory();
        std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建日志目录：{e}"))?;
        let now = Local::now();
        let file = dir.join(format!("suite-{}.log", now.format("%Y%m%d")));
        let _guard = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)
            .map_err(|e| format!("没法写日志：{e}"))?;
        writeln!(f, "{} [INFO] log enabled", now.format("%Y-%m-%d %H:%M:%S%.3f"))
            .map_err(|e| format!("没法写日志：{e}"))
    }

    pub fn try_open_folder() -> Result<(), String> {
        let dir = SettingsPaths::logs_directory();
        std::fs::create_dir_all(&dir).map_err(|e| format!("打不开日志文件夹：{e}"))?;
        let verb = to_wide_null("open");
        let path = to_wide_null(&dir.to_string_lossy());
        let h = unsafe {
            ShellExecuteW(
                Some(HWND::default()),
                PCWSTR(verb.as_ptr()),
                PCWSTR(path.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if (h.0 as usize) <= 32 {
            Err("打不开日志文件夹。".to_string())
        } else {
            Ok(())
        }
    }

    fn write_line(level: &str, message: &str) {
        let dir = SettingsPaths::logs_directory();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let now = Local::now();
        let file = dir.join(format!("suite-{}.log", now.format("%Y%m%d")));
        let _guard = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&file) {
            let _ = writeln!(f, "{} [{level}] {message}", now.format("%Y-%m-%d %H:%M:%S%.3f"));
        }
    }
}
