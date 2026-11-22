use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use suite_contracts::{AppSettings, CrashGuardState, SettingsPaths};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DegradeResult {
    pub degraded: bool,
    pub safe_mode: bool,
    pub message: Option<String>,
    pub last_start_label: String,
}

pub struct CrashGuard;

impl CrashGuard {
    pub const MSG_FX_OFF: &'static str = "上次异常退出，已暂时关闭任务栏效果。";
    pub const MSG_FLOAT: &'static str = "上次异常退出，网速已改回桌面悬浮窗。";
    pub const MSG_SAFE: &'static str = "已用安全设置启动。任务栏效果已暂时关闭；网速钉栏保持开启。";
    pub const MSG_WRITE_FAIL: &'static str = "没法写入安全设置，请检查磁盘权限。";

    pub fn load() -> CrashGuardState {
        Self::load_from(&SettingsPaths::crash_guard_path())
    }

    pub fn load_from(path: &Path) -> CrashGuardState {
        match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => CrashGuardState::default(),
        }
    }

    pub fn try_save(state: &CrashGuardState) -> Result<(), String> {
        Self::try_save_to(&SettingsPaths::crash_guard_path(), state)
    }

    pub fn try_save_to(path: &Path, state: &CrashGuardState) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("{} {e}", Self::MSG_WRITE_FAIL))?;
        }
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| format!("{} {e}", Self::MSG_WRITE_FAIL))?;
        std::fs::write(path, json).map_err(|e| format!("{} {e}", Self::MSG_WRITE_FAIL))
    }

    pub fn on_startup(settings: &mut AppSettings) -> DegradeResult {
        Self::on_startup_at(&SettingsPaths::crash_guard_path(), settings)
    }

    pub fn on_startup_at(path: &Path, settings: &mut AppSettings) -> DegradeResult {
        let mut state = Self::load_from(path);
        let now = now_unix_ms();
        let dirty = !state.clean_exit;
        let recent = state.last_start_unix_ms > 0 && (now - state.last_start_unix_ms) < 30_000;
        let mut crashes = if dirty { state.recent_crash_count + 1 } else { 0 };
        if dirty && recent {
            crashes = crashes.max(state.recent_crash_count + 1);
        }

        let mut messages: Vec<&'static str> = Vec::new();
        let safe = dirty && crashes >= 4;
        let mut degraded = false;

        if safe {
            if settings.taskbar_fx.enabled {
                settings.taskbar_fx.enabled = false;
                degraded = true;
            }
            messages.push(Self::MSG_SAFE);
        } else if dirty
            && state.taskbar_fx_was_enabled
            && settings.taskbar_fx.enabled
            && settings.advanced.taskbar_fx_auto_disable_on_fail
        {
            settings.taskbar_fx.enabled = false;
            messages.push(Self::MSG_FX_OFF);
            degraded = true;
        }

        state.clean_exit = false;
        state.taskbar_fx_was_enabled = settings.taskbar_fx.enabled;
        state.net_speed_embed_was_on = settings.net_speed.embed_in_taskbar && settings.net_speed.visible;
        state.last_start_unix_ms = now;
        state.recent_crash_count = if dirty { crashes } else { 0 };
        state.last_degrade = if messages.is_empty() {
            None
        } else {
            Some(messages.join(" "))
        };
        let _ = Self::try_save_to(path, &state);

        let message = if messages.is_empty() {
            None
        } else {
            Some(messages.join(" "))
        };
        let last_start_label = if degraded || safe || dirty {
            "已降级".to_string()
        } else {
            "正常".to_string()
        };

        DegradeResult {
            degraded,
            safe_mode: safe,
            message,
            last_start_label,
        }
    }

    pub fn mark_running_features(settings: &AppSettings) {
        Self::mark_running_features_at(&SettingsPaths::crash_guard_path(), settings);
    }

    pub fn mark_running_features_at(path: &Path, settings: &AppSettings) {
        let mut state = Self::load_from(path);
        state.clean_exit = false;
        state.taskbar_fx_was_enabled = settings.taskbar_fx.enabled;
        state.net_speed_embed_was_on = settings.net_speed.visible && settings.net_speed.embed_in_taskbar;
        let _ = Self::try_save_to(path, &state);
    }

    pub fn mark_clean_exit() {
        Self::mark_clean_exit_at(&SettingsPaths::crash_guard_path());
    }

    pub fn mark_clean_exit_at(path: &Path) {
        let mut state = Self::load_from(path);
        state.clean_exit = true;
        state.recent_crash_count = 0;
        let _ = Self::try_save_to(path, &state);
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_guard_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("suite-cg-test-{name}-{nanos}"))
            .join("crash-guard.json")
    }

    #[test]
    fn dirty_exit_disables_taskbar_fx_but_keeps_netspeed_embed() {
        let path = temp_guard_path("dirty");
        let dir = path.parent().unwrap().to_path_buf();
        let mut settings = AppSettings::default();
        settings.taskbar_fx.enabled = true;
        settings.net_speed.embed_in_taskbar = true;

        // First startup: clean
        let first = CrashGuard::on_startup_at(&path, &mut settings);
        assert!(!first.degraded);
        assert!(settings.taskbar_fx.enabled);

        // Simulate crash (no mark_clean_exit) -> second startup
        let second = CrashGuard::on_startup_at(&path, &mut settings);
        assert!(second.degraded);
        assert!(!settings.taskbar_fx.enabled);
        assert!(settings.net_speed.embed_in_taskbar);
        assert_eq!(second.last_start_label, "已降级");

        // Clean exit resets crash count
        CrashGuard::mark_clean_exit_at(&path);
        let state = CrashGuard::load_from(&path);
        assert!(state.clean_exit);
        assert_eq!(state.recent_crash_count, 0);

        let _ = std::fs::remove_dir_all(dir);
    }
}
