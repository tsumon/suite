use std::sync::atomic::{AtomicI32, AtomicU32};

use windows_core::GUID;

/// {B7C91E2A-4D53-4F80-9A1C-6E8F2B4D7C10}
pub const CLSID_TASKBAR_FX_TAP_SITE: GUID = GUID::from_u128(0xb7c91e2a_4d53_4f80_9a1c_6e8f2b4d7c10);

pub const K_TASKBAR_FX_TAP_SHM_NAME: &str = r"Local\Suite.TaskbarFx.Tap.Shm";
pub const K_TASKBAR_FX_TAP_READY_EVENT_NAME: &str = r"Local\Suite.TaskbarFx.Tap.Ready";
pub const K_TASKBAR_FX_TAP_APPLY_EVENT_NAME: &str = r"Local\Suite.TaskbarFx.Tap.Apply";

pub const K_TASKBAR_FX_TAP_MAGIC: u32 = 0x3158_4654; // TFX1
pub const K_TASKBAR_FX_TAP_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TapResultCode {
    Ok = 0,
    PendingFills = 1,
    AcrylicFallback = 2,
    Failed = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TaskbarFxResultCode {
    Ok = 0,
    ErrMissingExport = 1,
    ErrNoTray = 2,
    ErrModernXaml = 3,
    ErrCallFailed = 4,
    ErrBadMode = 5,
    ErrTapInit = 6,
}

/// Exact ABI layout of `TaskbarFxTapShm` from `src/TaskbarFx.Shared/TaskbarFxTapProtocol.h` (`#pragma pack(push, 8)`).
#[repr(C)]
pub struct TaskbarFxTapShm {
    pub magic: u32,
    pub version: u32,
    pub seq: AtomicU32,
    pub ack: AtomicU32,
    pub mode: AtomicI32,
    pub argb: AtomicU32,
    pub result: AtomicI32,
    pub ready: AtomicI32,
    pub fill_count: AtomicU32,
    pub host_pid: AtomicU32,
    pub explorer_pid: AtomicU32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn shared_memory_abi_matches_cpp_header() {
        assert_eq!(size_of::<TaskbarFxTapShm>(), 44);
        assert_eq!(offset_of!(TaskbarFxTapShm, magic), 0);
        assert_eq!(offset_of!(TaskbarFxTapShm, version), 4);
        assert_eq!(offset_of!(TaskbarFxTapShm, seq), 8);
        assert_eq!(offset_of!(TaskbarFxTapShm, ack), 12);
        assert_eq!(offset_of!(TaskbarFxTapShm, mode), 16);
        assert_eq!(offset_of!(TaskbarFxTapShm, argb), 20);
        assert_eq!(offset_of!(TaskbarFxTapShm, result), 24);
        assert_eq!(offset_of!(TaskbarFxTapShm, ready), 28);
        assert_eq!(offset_of!(TaskbarFxTapShm, fill_count), 32);
        assert_eq!(offset_of!(TaskbarFxTapShm, host_pid), 36);
        assert_eq!(offset_of!(TaskbarFxTapShm, explorer_pid), 40);
    }
}
