pub mod chrome;
pub mod copy;
pub mod paths;
pub mod settings;

pub use chrome::{AnnotateTool, AnnotateToolbar, CurveStyle, Placement, RateFormatter, StackPlacement, TaskbarSlotChrome};
pub use copy::{HotkeyCopy, NetSpeedCopy, TaskbarFxCopy};
pub use paths::SettingsPaths;
pub use settings::{
    AdvancedSettings, AppSettings, CaptureSettings, CrashGuardState, HotkeyBinding, NetSpeedSettings,
    SettingsJson, TaskbarAppearanceMode, TaskbarAppearanceModes, TaskbarFxSettings, ThemeKind, UpdateSettings,
};
