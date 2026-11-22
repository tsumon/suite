use std::path::{Path, PathBuf};

use suite_contracts::{AppSettings, SettingsJson, SettingsPaths};

pub struct SettingsStore {
    path: PathBuf,
    current: AppSettings,
}

impl SettingsStore {
    pub fn new() -> Self {
        Self {
            path: SettingsPaths::file_path(),
            current: AppSettings::create_default(),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self {
            path,
            current: AppSettings::create_default(),
        }
    }

    pub fn current(&self) -> &AppSettings {
        &self.current
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&mut self) -> &AppSettings {
        if !self.path.exists() {
            self.current = AppSettings::create_default();
            return &self.current;
        }
        match std::fs::read_to_string(&self.path) {
            Ok(raw) => match SettingsJson::deserialize(&raw) {
                Ok(parsed) => {
                    self.current = parsed;
                }
                Err(_) => {
                    self.current = AppSettings::create_default();
                }
            },
            Err(_) => {
                self.current = AppSettings::create_default();
            }
        }
        &self.current
    }

    pub fn save(&mut self, settings: AppSettings) -> Result<(), String> {
        self.current = settings;
        let dir = self
            .path
            .parent()
            .ok_or_else(|| "Missing settings directory".to_string())?;
        std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create settings dir: {e}"))?;
        let json = SettingsJson::serialize(&self.current)
            .map_err(|e| format!("Failed to serialize settings: {e}"))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| format!("Failed to write temp settings: {e}"))?;
        if let Err(err) = std::fs::rename(&tmp, &self.path) {
            // Fallback overwrite if destination was momentarily locked
            let _ = std::fs::copy(&tmp, &self.path);
            let _ = std::fs::remove_file(&tmp);
            if !self.path.exists() {
                return Err(format!("Failed to save settings: {err}"));
            }
        }
        Ok(())
    }

    pub fn update<F>(&mut self, mutator: F) -> Result<(), String>
    where
        F: FnOnce(&mut AppSettings),
    {
        let mut next = self.current.clone();
        mutator(&mut next);
        self.save(next)
    }
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self::new()
    }
}
