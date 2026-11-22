use serde::{Deserialize, Deserializer, Serialize};

pub const CURRENT_SCHEMA_VERSION: i32 = 4;

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::<T>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn default_true() -> bool {
    true
}

fn default_schema_version() -> i32 {
    CURRENT_SCHEMA_VERSION
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum ThemeKind {
    Dark = 0,
    #[default]
    Light = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum TaskbarAppearanceMode {
    #[default]
    Normal = 0,
    Opaque = 1,
    Clear = 2,
    Acrylic = 3,
}

pub struct TaskbarAppearanceModes;

impl TaskbarAppearanceModes {
    pub const NORMAL: &'static str = "normal";
    pub const OPAQUE: &'static str = "opaque";
    pub const CLEAR: &'static str = "clear";
    pub const ACRYLIC: &'static str = "acrylic";

    pub fn to_wire(mode: TaskbarAppearanceMode) -> &'static str {
        match mode {
            TaskbarAppearanceMode::Opaque => Self::OPAQUE,
            TaskbarAppearanceMode::Clear => Self::CLEAR,
            TaskbarAppearanceMode::Acrylic => Self::ACRYLIC,
            TaskbarAppearanceMode::Normal => Self::NORMAL,
        }
    }

    pub fn try_parse(value: Option<&str>) -> Option<TaskbarAppearanceMode> {
        match value.unwrap_or("").trim().to_ascii_lowercase().as_str() {
            Self::OPAQUE => Some(TaskbarAppearanceMode::Opaque),
            Self::CLEAR => Some(TaskbarAppearanceMode::Clear),
            Self::ACRYLIC => Some(TaskbarAppearanceMode::Acrylic),
            Self::NORMAL | "" => Some(TaskbarAppearanceMode::Normal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBinding {
    #[serde(default)]
    pub control: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub win: bool,
    #[serde(default = "HotkeyBinding::default_virtual_key_val")]
    pub virtual_key: i32,
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            control: false,
            shift: false,
            alt: false,
            win: false,
            virtual_key: Self::DEFAULT_VIRTUAL_KEY,
        }
    }
}

impl HotkeyBinding {
    pub const DEFAULT_VIRTUAL_KEY: i32 = 0x70; // VK_F1
    pub const DEFAULT_COLOR_PICK_VIRTUAL_KEY: i32 = 0x71; // VK_F2
    pub const PIN_CLIPBOARD_VIRTUAL_KEY: i32 = 0x72; // VK_F3
    pub const DEFAULT_SCROLL_CAPTURE_VIRTUAL_KEY: i32 = 0x73; // VK_F4

    fn default_virtual_key_val() -> i32 {
        Self::DEFAULT_VIRTUAL_KEY
    }

    pub fn default_color_pick() -> Self {
        Self {
            virtual_key: Self::DEFAULT_COLOR_PICK_VIRTUAL_KEY,
            ..Default::default()
        }
    }

    pub fn default_scroll_capture() -> Self {
        Self {
            virtual_key: Self::DEFAULT_SCROLL_CAPTURE_VIRTUAL_KEY,
            ..Default::default()
        }
    }

    pub fn default_pin_click_through() -> Self {
        Self {
            control: true,
            shift: true,
            alt: false,
            win: false,
            virtual_key: 0x51, // 'Q'
        }
    }

    pub fn is_disabled(&self) -> bool {
        self.virtual_key <= 0
    }

    pub fn to_native_modifiers(&self, no_repeat: bool) -> u32 {
        let mut mods = 0u32;
        if self.alt {
            mods |= 0x0001; // MOD_ALT
        }
        if self.control {
            mods |= 0x0002; // MOD_CONTROL
        }
        if self.shift {
            mods |= 0x0004; // MOD_SHIFT
        }
        if self.win {
            mods |= 0x0008; // MOD_WIN
        }
        if no_repeat {
            mods |= 0x4000; // MOD_NOREPEAT
        }
        mods
    }

    pub fn to_display_string(&self) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(5);
        if self.control {
            parts.push("Ctrl".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.win {
            parts.push("Win".to_string());
        }
        parts.push(Self::virtual_key_name(self.virtual_key));
        parts.join("+")
    }

    pub fn virtual_key_name(vk: i32) -> String {
        match vk {
            0x70..=0x87 => format!("F{}", vk - 0x6F),
            0x30..=0x39 | 0x41..=0x5A => ((vk as u8) as char).to_string(),
            0x20 => "Space".to_string(),
            0x2D => "Insert".to_string(),
            0x2E => "Delete".to_string(),
            0x24 => "Home".to_string(),
            0x23 => "End".to_string(),
            0x21 => "PageUp".to_string(),
            0x22 => "PageDown".to_string(),
            _ => format!("VK_0x{:02X}", vk),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetSpeedSettings {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub embed_in_taskbar: bool,
    #[serde(default)]
    pub embed_secondary: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_alias: Option<String>,
    #[serde(default = "NetSpeedSettings::default_coord")]
    pub left: f64,
    #[serde(default = "NetSpeedSettings::default_coord")]
    pub top: f64,
    #[serde(default = "NetSpeedSettings::default_font_size")]
    pub font_size_dip: f64,
    #[serde(default = "default_true")]
    pub show_download: bool,
    #[serde(default = "default_true")]
    pub show_upload: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub down_color_argb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up_color_argb: Option<u32>,
    #[serde(default = "default_true")]
    pub background_transparent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_argb: Option<u32>,
    #[serde(default)]
    pub show_border: bool,
    #[serde(default = "default_true")]
    pub two_line: bool,
    #[serde(default)]
    pub bold: bool,
}

impl Default for NetSpeedSettings {
    fn default() -> Self {
        Self {
            visible: true,
            embed_in_taskbar: true,
            embed_secondary: false,
            if_index: None,
            adapter_alias: None,
            left: Self::default_coord(),
            top: Self::default_coord(),
            font_size_dip: Self::DEFAULT_FONT_SIZE_DIP,
            show_download: true,
            show_upload: true,
            down_color_argb: None,
            up_color_argb: None,
            background_transparent: true,
            background_argb: None,
            show_border: false,
            two_line: true,
            bold: false,
        }
    }
}

impl NetSpeedSettings {
    pub const DEFAULT_FONT_SIZE_DIP: f64 = 13.0;
    pub const MIN_FONT_SIZE_DIP: f64 = 10.0;
    pub const MAX_FONT_SIZE_DIP: f64 = 18.0;

    fn default_coord() -> f64 {
        64.0
    }

    fn default_font_size() -> f64 {
        Self::DEFAULT_FONT_SIZE_DIP
    }

    pub fn normalize(&mut self) {
        if !self.font_size_dip.is_finite() {
            self.font_size_dip = Self::DEFAULT_FONT_SIZE_DIP;
        }
        self.font_size_dip = self
            .font_size_dip
            .clamp(Self::MIN_FONT_SIZE_DIP, Self::MAX_FONT_SIZE_DIP);
        if !self.show_download && !self.show_upload {
            self.show_download = true;
            self.show_upload = true;
        }
    }

    pub fn visible_line_count(&self) -> usize {
        if !self.two_line {
            return usize::from(self.show_download || self.show_upload);
        }
        usize::from(self.show_download) + usize::from(self.show_upload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSettings {
    #[serde(default)]
    pub save_file_after_capture: bool,
    #[serde(default)]
    pub pin_after_capture: bool,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub save_directory: String,
    #[serde(default = "default_true")]
    pub show_magnifier: bool,
    #[serde(default = "default_true")]
    pub history_enabled: bool,
    #[serde(default = "CaptureSettings::default_history_max")]
    pub history_max: i32,
    #[serde(default = "HotkeyBinding::default_color_pick", deserialize_with = "deserialize_color_pick_or_default")]
    pub color_pick_hotkey: HotkeyBinding,
    #[serde(default = "HotkeyBinding::default_pin_click_through", deserialize_with = "deserialize_click_through_or_default")]
    pub pin_click_through_hotkey: HotkeyBinding,
    #[serde(default = "HotkeyBinding::default_scroll_capture", deserialize_with = "deserialize_scroll_capture_or_default")]
    pub scroll_capture_hotkey: HotkeyBinding,
}

fn deserialize_color_pick_or_default<'de, D>(deserializer: D) -> Result<HotkeyBinding, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<HotkeyBinding>::deserialize(deserializer)?.unwrap_or_else(HotkeyBinding::default_color_pick))
}

fn deserialize_click_through_or_default<'de, D>(deserializer: D) -> Result<HotkeyBinding, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<HotkeyBinding>::deserialize(deserializer)?.unwrap_or_else(HotkeyBinding::default_pin_click_through))
}

fn deserialize_scroll_capture_or_default<'de, D>(deserializer: D) -> Result<HotkeyBinding, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<HotkeyBinding>::deserialize(deserializer)?.unwrap_or_else(HotkeyBinding::default_scroll_capture))
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            save_file_after_capture: false,
            pin_after_capture: false,
            save_directory: String::new(),
            show_magnifier: true,
            history_enabled: true,
            history_max: Self::default_history_max(),
            color_pick_hotkey: HotkeyBinding::default_color_pick(),
            pin_click_through_hotkey: HotkeyBinding::default_pin_click_through(),
            scroll_capture_hotkey: HotkeyBinding::default_scroll_capture(),
        }
    }
}

impl CaptureSettings {
    fn default_history_max() -> i32 {
        30
    }

    pub fn normalize(&mut self) {
        if self.history_max < 1 {
            self.history_max = 30;
        }
        if self.history_max > 200 {
            self.history_max = 200;
        }
        if !(0..=0xFE).contains(&self.color_pick_hotkey.virtual_key) {
            self.color_pick_hotkey = HotkeyBinding::default_color_pick();
        }
        if !(0..=0xFE).contains(&self.pin_click_through_hotkey.virtual_key) {
            self.pin_click_through_hotkey = HotkeyBinding::default_pin_click_through();
        }
        if !(0..=0xFE).contains(&self.scroll_capture_hotkey.virtual_key) {
            self.scroll_capture_hotkey = HotkeyBinding::default_scroll_capture();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarFxSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "TaskbarFxSettings::default_mode", deserialize_with = "deserialize_mode_or_default")]
    pub mode: String,
    #[serde(default = "TaskbarFxSettings::default_argb")]
    pub argb: u32,
    #[serde(default = "default_true")]
    pub keep_appearance_if_suite_exits: bool,
}

fn deserialize_mode_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_else(TaskbarFxSettings::default_mode))
}

impl Default for TaskbarFxSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: Self::default_mode(),
            argb: Self::DEFAULT_ARGB,
            keep_appearance_if_suite_exits: true,
        }
    }
}

impl TaskbarFxSettings {
    pub const DEFAULT_ARGB: u32 = 0xCC1F_1F1F;

    fn default_mode() -> String {
        TaskbarAppearanceModes::ACRYLIC.to_string()
    }

    fn default_argb() -> u32 {
        Self::DEFAULT_ARGB
    }

    pub fn parsed_mode(&self) -> TaskbarAppearanceMode {
        TaskbarAppearanceModes::try_parse(Some(&self.mode)).unwrap_or(TaskbarAppearanceMode::Acrylic)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettings {
    #[serde(default = "UpdateSettings::default_channel", deserialize_with = "deserialize_channel_or_default")]
    pub channel: String,
    #[serde(default = "UpdateSettings::default_owner", deserialize_with = "deserialize_owner_or_default")]
    pub owner: String,
    #[serde(default = "UpdateSettings::default_repo", deserialize_with = "deserialize_repo_or_default")]
    pub repo: String,
}

fn deserialize_channel_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_else(UpdateSettings::default_channel))
}

fn deserialize_owner_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_else(UpdateSettings::default_owner))
}

fn deserialize_repo_or_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_else(UpdateSettings::default_repo))
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            channel: Self::default_channel(),
            owner: Self::default_owner(),
            repo: Self::default_repo(),
        }
    }
}

impl UpdateSettings {
    pub const CHANNEL_STABLE: &'static str = "stable";
    pub const CHANNEL_PREVIEW: &'static str = "preview";

    fn default_channel() -> String {
        Self::CHANNEL_STABLE.to_string()
    }

    fn default_owner() -> String {
        "tsumon".to_string()
    }

    fn default_repo() -> String {
        "suite".to_string()
    }

    pub fn normalize(&mut self) {
        if self.channel.eq_ignore_ascii_case(Self::CHANNEL_PREVIEW) {
            self.channel = Self::CHANNEL_PREVIEW.to_string();
        } else {
            self.channel = Self::CHANNEL_STABLE.to_string();
        }
        let owner = self.owner.trim();
        self.owner = if owner.is_empty() {
            "tsumon".to_string()
        } else {
            owner.to_string()
        };
        let repo = self.repo.trim();
        self.repo = if repo.is_empty() {
            "suite".to_string()
        } else {
            repo.to_string()
        };
    }

    pub fn channel_display_zh(&self) -> &'static str {
        if self.channel.eq_ignore_ascii_case(Self::CHANNEL_PREVIEW) {
            "预览"
        } else {
            "正式"
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedSettings {
    #[serde(default)]
    pub logging_enabled: bool,
    #[serde(default = "default_true")]
    pub taskbar_fx_auto_disable_on_fail: bool,
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            logging_enabled: false,
            taskbar_fx_auto_disable_on_fail: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_schema_version")]
    pub schema_version: i32,
    #[serde(default)]
    pub start_with_windows: bool,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub hotkey: HotkeyBinding,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub net_speed: NetSpeedSettings,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub capture: CaptureSettings,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub taskbar_fx: TaskbarFxSettings,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub update: UpdateSettings,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub advanced: AdvancedSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            start_with_windows: false,
            hotkey: HotkeyBinding::default(),
            net_speed: NetSpeedSettings::default(),
            capture: CaptureSettings::default(),
            taskbar_fx: TaskbarFxSettings::default(),
            update: UpdateSettings::default(),
            advanced: AdvancedSettings::default(),
        }
    }
}

impl AppSettings {
    pub fn create_default() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashGuardState {
    #[serde(default = "default_true")]
    pub clean_exit: bool,
    #[serde(default)]
    pub taskbar_fx_was_enabled: bool,
    #[serde(default)]
    pub net_speed_embed_was_on: bool,
    #[serde(default)]
    pub last_start_unix_ms: i64,
    #[serde(default)]
    pub recent_crash_count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_degrade: Option<String>,
}

impl Default for CrashGuardState {
    fn default() -> Self {
        Self {
            clean_exit: true,
            taskbar_fx_was_enabled: false,
            net_speed_embed_was_on: false,
            last_start_unix_ms: 0,
            recent_crash_count: 0,
            last_degrade: None,
        }
    }
}

pub struct SettingsJson;

impl SettingsJson {
    pub fn serialize(settings: &AppSettings) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(settings)
    }

    pub fn deserialize(raw: &str) -> Result<AppSettings, serde_json::Error> {
        let stripped = strip_jsonc_comments_and_trailing_commas(raw);
        let mut settings: AppSettings = serde_json::from_str(&stripped)?;
        settings.net_speed.normalize();
        settings.capture.save_directory = settings.capture.save_directory.clone();
        settings.capture.normalize();
        settings.update.normalize();

        if settings.taskbar_fx.mode.trim().is_empty()
            || TaskbarAppearanceModes::try_parse(Some(&settings.taskbar_fx.mode)).is_none()
        {
            settings.taskbar_fx.mode = TaskbarAppearanceModes::ACRYLIC.to_string();
        }
        if settings.hotkey.virtual_key <= 0 || settings.hotkey.virtual_key > 0xFE {
            settings.hotkey.virtual_key = HotkeyBinding::DEFAULT_VIRTUAL_KEY;
        }

        if settings.schema_version < 2 {
            settings.net_speed.embed_in_taskbar = true;
            settings.schema_version = 2;
        }

        if settings.schema_version < 3 {
            settings.hotkey = HotkeyBinding::default();
            settings.capture.pin_after_capture = false;
            settings.schema_version = 3;
        }

        if settings.schema_version < 4 {
            if settings.capture.history_max <= 0 {
                settings.capture.history_max = 30;
            }
            settings.schema_version = CURRENT_SCHEMA_VERSION;
        }

        if settings.schema_version <= 0 {
            settings.schema_version = CURRENT_SCHEMA_VERSION;
        }

        settings.capture.normalize();
        settings.update.normalize();
        Ok(settings)
    }
}

/// Strip `// ...`, `/* ... */` comments and trailing commas from JSONC safely outside string literals.
fn strip_jsonc_comments_and_trailing_commas(input: &str) -> String {
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);
    let bytes = input.as_bytes();
    let mut no_comments = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            no_comments.push(b);
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if b == b'"' {
            in_string = true;
            no_comments.push(b);
            i += 1;
        } else if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            no_comments.push(b);
            i += 1;
        }
    }

    // Second pass: drop trailing commas before `}` or `]`.
    let mut out = Vec::with_capacity(no_comments.len());
    let mut j = 0;
    in_string = false;
    escaped = false;
    while j < no_comments.len() {
        let b = no_comments[j];
        if in_string {
            out.push(b);
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            j += 1;
            continue;
        }

        if b == b'"' {
            in_string = true;
            out.push(b);
            j += 1;
        } else if b == b',' {
            let mut k = j + 1;
            while k < no_comments.len() && no_comments[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < no_comments.len() && (no_comments[k] == b'}' || no_comments[k] == b']') {
                j += 1; // skip trailing comma
            } else {
                out.push(b);
                j += 1;
            }
        } else {
            out.push(b);
            j += 1;
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_live_schema4_with_extra_and_comments() {
        let live_json = r#"{
          // comment inside jsonc
          "schemaVersion": 4,
          "startWithWindows": true,
          "hotkey": {
            "control": false,
            "shift": false,
            "alt": false,
            "win": false,
            "virtualKey": 112,
            "isDisabled": false
          },
          "netSpeed": {
            "visible": true,
            "embedInTaskbar": true,
            "embedSecondary": true,
            "ifIndex": 9,
            "adapterAlias": "WLAN",
            "left": 2864.8,
            "top": 1603.2,
            "fontSizeDip": 14,
            "showDownload": true,
            "showUpload": true,
            "backgroundTransparent": true,
            "backgroundArgb": 4280518009,
            "showBorder": false,
            "twoLine": true,
            "bold": false
          },
          "capture": {
            "saveFileAfterCapture": false,
            "pinAfterCapture": false,
            "saveDirectory": "",
            "showMagnifier": true,
            "historyEnabled": true,
            "historyMax": 30,
            "colorPickHotkey": { "virtualKey": 113 },
            "pinClickThroughHotkey": { "control": true, "shift": true, "virtualKey": 81 },
            "scrollCaptureHotkey": { "virtualKey": 115 }
          },
          "taskbarFx": {
            "enabled": true,
            "mode": "clear",
            "argb": 2039583,
            "keepAppearanceIfSuiteExits": true,
            "parsedMode": 2
          },
          "update": {
            "channel": "stable",
            "owner": "tsumon",
            "repo": "suite"
          },
          "advanced": {
            "loggingEnabled": true,
            "taskbarFxAutoDisableOnFail": true,
          }
        }"#;

        let parsed = SettingsJson::deserialize(live_json).expect("live json should parse");
        assert_eq!(parsed.schema_version, 4);
        assert!(parsed.start_with_windows);
        assert_eq!(parsed.hotkey.to_display_string(), "F1");
        assert_eq!(parsed.net_speed.if_index, Some(9));
        assert_eq!(parsed.net_speed.adapter_alias.as_deref(), Some("WLAN"));
        assert_eq!(parsed.taskbar_fx.parsed_mode(), TaskbarAppearanceMode::Clear);
        assert_eq!(parsed.capture.pin_click_through_hotkey.to_display_string(), "Ctrl+Shift+Q");
    }

    #[test]
    fn migrate_v1_settings_and_normalize() {
        let v1 = r#"{
          "schemaVersion": 1,
          "hotkey": { "control": true, "virtualKey": 65 },
          "netSpeed": { "embedInTaskbar": false, "fontSizeDip": 99, "showDownload": false, "showUpload": false },
          "capture": { "pinAfterCapture": true, "historyMax": -5, "scrollCaptureHotkey": null },
          "taskbarFx": { "mode": "bogus" }
        }"#;

        let migrated = SettingsJson::deserialize(v1).expect("v1 should migrate");
        assert_eq!(migrated.schema_version, 4);
        assert!(migrated.net_speed.embed_in_taskbar);
        assert_eq!(migrated.net_speed.font_size_dip, 18.0);
        assert!(migrated.net_speed.show_download && migrated.net_speed.show_upload);
        assert_eq!(migrated.hotkey.virtual_key, HotkeyBinding::DEFAULT_VIRTUAL_KEY);
        assert!(!migrated.capture.pin_after_capture);
        assert_eq!(migrated.capture.history_max, 30);
        assert_eq!(migrated.capture.scroll_capture_hotkey.virtual_key, 0x73);
        assert_eq!(migrated.taskbar_fx.mode, "acrylic");
    }
}
