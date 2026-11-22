pub mod protocol;
pub mod swca;
pub mod win11_tap;

use std::sync::Mutex;
use std::time::Duration;

use suite_contracts::{TaskbarAppearanceMode, TaskbarAppearanceModes, TaskbarFxCopy, TaskbarFxSettings};
use suite_platform::{TaskbarKind, TaskbarProbe};

use crate::protocol::TaskbarFxResultCode;
use crate::swca::apply_classic_swca;
use crate::win11_tap::Win11TapClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskbarFxStatus {
    pub enabled: bool,
    pub path: &'static str,
    pub mode: String,
    pub argb: u32,
    pub message: String,
    pub last_error: Option<String>,
    pub result_code: i32,
}

impl TaskbarFxStatus {
    pub const PATH_UNAVAILABLE: &'static str = "unavailable";
    pub const PATH_WIN10_SWCA: &'static str = "win10-swca";
    pub const PATH_WIN11_XAML: &'static str = "win11-xaml";
}

pub struct TaskbarFxEngine {
    tap: Mutex<Win11TapClient>,
}

impl TaskbarFxEngine {
    pub fn new() -> Self {
        Self {
            tap: Mutex::new(Win11TapClient::new()),
        }
    }

    pub fn probe_kind(&self) -> TaskbarKind {
        let mut kind = TaskbarProbe::detect();
        if kind != TaskbarKind::ModernXaml && kind != TaskbarKind::NotFound {
            std::thread::sleep(Duration::from_millis(1500));
            let again = TaskbarProbe::detect();
            if again == TaskbarKind::ModernXaml || again == TaskbarKind::NotFound {
                kind = again;
            }
        }
        kind
    }

    pub fn apply(&self, settings: &TaskbarFxSettings, _shutdown_when_disabled: bool) -> TaskbarFxStatus {
        let enabled = settings.enabled;
        let mode = if !enabled {
            TaskbarAppearanceMode::Normal
        } else {
            settings.parsed_mode()
        };
        let argb = settings.argb;
        let mode_wire = TaskbarAppearanceModes::to_wire(mode).to_string();

        let kind = self.probe_kind();
        let path = match kind {
            TaskbarKind::ModernXaml => TaskbarFxStatus::PATH_WIN11_XAML,
            TaskbarKind::ClassicWin32 => TaskbarFxStatus::PATH_WIN10_SWCA,
            TaskbarKind::NotFound => {
                let err = TaskbarFxCopy::apply_failed(Some("找不到任务栏。"));
                return TaskbarFxStatus {
                    enabled,
                    path: TaskbarFxStatus::PATH_UNAVAILABLE,
                    mode: mode_wire,
                    argb,
                    message: err.clone(),
                    last_error: Some(err),
                    result_code: TaskbarFxResultCode::ErrNoTray as i32,
                };
            }
        };

        let effective_mode = if enabled && mode != TaskbarAppearanceMode::Normal {
            mode
        } else {
            TaskbarAppearanceMode::Normal
        };

        let (code, detail) = match kind {
            TaskbarKind::ModernXaml => {
                let mut tap = self.tap.lock().unwrap_or_else(|e| e.into_inner());
                tap.apply(effective_mode, argb)
            }
            TaskbarKind::ClassicWin32 => apply_classic_swca(effective_mode, argb),
            TaskbarKind::NotFound => unreachable!(),
        };

        if code == TaskbarFxResultCode::Ok {
            let msg = if !enabled || effective_mode == TaskbarAppearanceMode::Normal {
                TaskbarFxCopy::CLOSED.to_string()
            } else {
                TaskbarFxCopy::APPLIED.to_string()
            };
            TaskbarFxStatus {
                enabled,
                path,
                mode: mode_wire,
                argb,
                message: msg,
                last_error: None,
                result_code: 0,
            }
        } else {
            let short = match code {
                TaskbarFxResultCode::ErrMissingExport => "系统不支持这种任务栏外观。".to_string(),
                TaskbarFxResultCode::ErrNoTray => "找不到任务栏。".to_string(),
                TaskbarFxResultCode::ErrBadMode => "档位无效。".to_string(),
                TaskbarFxResultCode::ErrTapInit => {
                    if detail.contains("安全软件") {
                        detail
                    } else {
                        "任务栏效果被安全软件拦住，或 Win11 辅助模块没编出来。".to_string()
                    }
                }
                _ => {
                    if detail.trim().is_empty() {
                        "无法改任务栏外观。".to_string()
                    } else {
                        detail
                    }
                }
            };
            let err = TaskbarFxCopy::apply_failed(Some(&short));
            TaskbarFxStatus {
                enabled,
                path: if kind == TaskbarKind::ModernXaml {
                    TaskbarFxStatus::PATH_WIN11_XAML
                } else {
                    TaskbarFxStatus::PATH_UNAVAILABLE
                },
                mode: mode_wire,
                argb,
                message: err.clone(),
                last_error: Some(err),
                result_code: code as i32,
            }
        }
    }

    pub fn reset(&self) -> TaskbarFxStatus {
        let off = TaskbarFxSettings {
            enabled: false,
            mode: TaskbarAppearanceModes::NORMAL.to_string(),
            ..Default::default()
        };
        self.apply(&off, true)
    }
}

impl Default for TaskbarFxEngine {
    fn default() -> Self {
        Self::new()
    }
}
