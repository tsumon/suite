pub mod broadcast;
pub mod constants;
pub mod crash_guard;
pub mod integrity;
pub mod log;
pub mod registry;
pub mod settings_store;
pub mod single_instance;
pub mod taskbar;

pub use broadcast::send_immersive_color_set;
pub use crash_guard::{CrashGuard, DegradeResult};
pub use integrity::is_elevated_administrator;
pub use log::SuiteLog;
pub use registry::{PersonalizeDwords, PersonalizeRegistry, RunKeyService};
pub use settings_store::SettingsStore;
pub use single_instance::SingleInstanceGuard;
pub use taskbar::{TaskbarEmbed, TaskbarKind, TaskbarProbe, TrayRect};
