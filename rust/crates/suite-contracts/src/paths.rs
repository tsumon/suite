use std::path::PathBuf;

pub struct SettingsPaths;

impl SettingsPaths {
    pub const APP_FOLDER_NAME: &'static str = "Suite";
    pub const FILE_NAME: &'static str = "settings.json";
    pub const CRASH_GUARD_FILE_NAME: &'static str = "crash-guard.json";
    pub const HISTORY_FOLDER_NAME: &'static str = "History";
    pub const LOGS_FOLDER_NAME: &'static str = "logs";

    /// Base data directory. Honors `SUITE_DATA_DIR` override for isolated testing.
    pub fn directory_path() -> PathBuf {
        if let Ok(override_dir) = std::env::var("SUITE_DATA_DIR") {
            let trimmed = override_dir.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }

        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let trimmed = local_app_data.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join(Self::APP_FOLDER_NAME);
            }
        }

        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let trimmed = user_profile.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed)
                    .join("AppData")
                    .join("Local")
                    .join(Self::APP_FOLDER_NAME);
            }
        }

        std::env::temp_dir().join(Self::APP_FOLDER_NAME)
    }

    pub fn file_path() -> PathBuf {
        Self::directory_path().join(Self::FILE_NAME)
    }

    pub fn crash_guard_path() -> PathBuf {
        Self::directory_path().join(Self::CRASH_GUARD_FILE_NAME)
    }

    pub fn capture_log_path() -> PathBuf {
        Self::directory_path().join("capture.log")
    }

    pub fn logs_directory() -> PathBuf {
        Self::directory_path().join(Self::LOGS_FOLDER_NAME)
    }

    pub fn default_capture_directory() -> PathBuf {
        if let Ok(override_dir) = std::env::var("SUITE_DATA_DIR") {
            let trimmed = override_dir.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join("Captures");
            }
        }

        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let trimmed = user_profile.trim();
            if !trimmed.is_empty() {
                let pictures = PathBuf::from(trimmed).join("Pictures");
                return pictures.join(Self::APP_FOLDER_NAME);
            }
        }

        Self::directory_path().join("Captures")
    }

    pub fn resolve_capture_directory(configured: Option<&str>) -> PathBuf {
        match configured {
            Some(s) if !s.trim().is_empty() => PathBuf::from(s.trim()),
            _ => Self::default_capture_directory(),
        }
    }

    pub fn resolve_history_directory(configured_capture_dir: Option<&str>) -> PathBuf {
        Self::resolve_capture_directory(configured_capture_dir).join(Self::HISTORY_FOLDER_NAME)
    }
}
