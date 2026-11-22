use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local, Utc};
use suite_contracts::{AppSettings, SettingsPaths};

use crate::clipboard::ImageClipboard;
use crate::pixel::PixelBuffer;
use crate::png_io::{load_png_file, save_png_to_path};

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub modified_utc: DateTime<Utc>,
}

pub struct ScreenshotHistory;

impl ScreenshotHistory {
    pub const EMPTY: &'static str = "还没有截图记录。";
    pub const OPEN_FAIL: &'static str = "打不开截图历史。";
    pub const MISSING: &'static str = "这张图找不到了。";
    pub const PIN_FAIL: &'static str = "没钉上，请再试一次。";
    pub const COPIED: &'static str = "已复制。";
    pub const CLEARED: &'static str = "已清空历史。";
    pub const MAX_AGE_DAYS: u64 = 7;

    pub fn directory_for(settings: &AppSettings) -> PathBuf {
        let dir_opt = if settings.capture.save_directory.trim().is_empty() {
            None
        } else {
            Some(settings.capture.save_directory.as_str())
        };
        SettingsPaths::resolve_history_directory(dir_opt)
    }

    pub fn list(settings: &AppSettings) -> Vec<HistoryEntry> {
        let dir = Self::directory_for(settings);
        if !dir.is_dir() {
            return Vec::new();
        }
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(Self::MAX_AGE_DAYS * 86_400))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let max = settings.capture.history_max.clamp(1, 200) as usize;

        let mut entries: Vec<(SystemTime, PathBuf)> = Vec::new();
        for item in read_dir.flatten() {
            let path = item.path();
            if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("png")) != Some(true) {
                continue;
            }
            if let Ok(meta) = item.metadata()
                && meta.is_file()
                && let Ok(mod_time) = meta.modified()
                && mod_time >= cutoff
            {
                entries.push((mod_time, path));
            }
        }
        entries.sort_by_key(|a| std::cmp::Reverse(a.0));
        entries.truncate(max);
        entries
            .into_iter()
            .map(|(t, path)| HistoryEntry {
                path,
                modified_utc: DateTime::<Utc>::from(t),
            })
            .collect()
    }

    pub fn try_add(buffer: &PixelBuffer, settings: &AppSettings) -> Option<PathBuf> {
        if !settings.capture.history_enabled || buffer.width <= 0 || buffer.height <= 0 {
            return None;
        }
        let dir = Self::directory_for(settings);
        if std::fs::create_dir_all(&dir).is_err() {
            return None;
        }
        let name = format!("hist-{}.png", Local::now().format("%Y%m%d-%H%M%S-%3f"));
        let path = dir.join(name);
        if save_png_to_path(buffer, &path).is_err() {
            return None;
        }
        Self::prune(settings);
        Some(path)
    }

    pub fn prune(settings: &AppSettings) {
        let dir = Self::directory_for(settings);
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            return;
        };
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(Self::MAX_AGE_DAYS * 86_400))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let max = settings.capture.history_max.clamp(1, 200) as usize;
        let mut files: Vec<(SystemTime, PathBuf)> = read_dir
            .flatten()
            .filter_map(|item| {
                let p = item.path();
                if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("png")) != Some(true) {
                    return None;
                }
                let mod_t = item.metadata().ok()?.modified().ok()?;
                Some((mod_t, p))
            })
            .collect();
        files.sort_by_key(|a| std::cmp::Reverse(a.0));
        for (i, (mod_t, p)) in files.into_iter().enumerate() {
            if i >= max || mod_t < cutoff {
                let _ = Self::try_delete(&p);
            }
        }
    }

    pub fn try_delete(path: &Path) -> bool {
        if path.exists() {
            std::fs::remove_file(path).is_ok()
        } else {
            true
        }
    }

    pub fn clear_all(settings: &AppSettings) -> bool {
        let dir = Self::directory_for(settings);
        if !dir.is_dir() {
            return true;
        }
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            return false;
        };
        let mut ok = true;
        for item in read_dir.flatten() {
            let p = item.path();
            if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("png")) == Some(true)
                && !Self::try_delete(&p)
            {
                ok = false;
            }
        }
        ok
    }

    pub fn load_png(path: &Path) -> Result<PixelBuffer, String> {
        if !path.is_file() {
            return Err(Self::MISSING.to_string());
        }
        load_png_file(path).map_err(|_| Self::MISSING.to_string())
    }

    pub fn try_copy_file_to_clipboard(path: &Path) -> Result<(), String> {
        let buf = Self::load_png(path)?;
        ImageClipboard::copy(&buf)
    }
}
