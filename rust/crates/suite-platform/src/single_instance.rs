use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

use crate::constants::{SINGLE_INSTANCE_MUTEX_NAME, to_wide_null};

pub struct SingleInstanceGuard {
    handle: HANDLE,
}

impl SingleInstanceGuard {
    /// Acquire the single-instance named mutex. Honors `SUITE_MUTEX_NAME` for isolated testing.
    pub fn try_acquire() -> Option<Self> {
        let name = std::env::var("SUITE_MUTEX_NAME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| SINGLE_INSTANCE_MUTEX_NAME.to_string());
        Self::try_acquire_named(&name)
    }

    pub fn try_acquire_named(mutex_name: &str) -> Option<Self> {
        let wide = to_wide_null(mutex_name);
        unsafe {
            let handle = CreateMutexW(None, true, PCWSTR(wide.as_ptr())).ok()?;
            if handle.is_invalid() {
                return None;
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                return None;
            }
            Some(Self { handle })
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = windows::Win32::System::Threading::ReleaseMutex(self.handle);
                let _ = CloseHandle(self.handle);
            }
        }
    }
}
