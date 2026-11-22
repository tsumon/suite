use std::path::Path;

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::PCWSTR;

use crate::constants::{
    APPS_USE_LIGHT_THEME, PERSONALIZE_SUBKEY, RUN_SUBKEY, RUN_VALUE_NAME, SYSTEM_USES_LIGHT_THEME,
    to_wide_null,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonalizeDwords {
    pub apps_use_light: bool,
    pub system_uses_light: bool,
}

impl Default for PersonalizeDwords {
    fn default() -> Self {
        Self {
            apps_use_light: true,
            system_uses_light: true,
        }
    }
}

pub struct PersonalizeRegistry;

impl PersonalizeRegistry {
    pub fn try_read() -> Result<PersonalizeDwords, String> {
        let subkey = to_wide_null(PERSONALIZE_SUBKEY);
        let mut hkey = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                Some(0),
                KEY_READ,
                &mut hkey,
            )
        };
        if status != ERROR_SUCCESS {
            if status.0 == 2 {
                return Ok(PersonalizeDwords::default());
            }
            return Err(format!("RegOpenKeyExW failed ({})", status.0));
        }

        let apps = read_dword_as_light(hkey, APPS_USE_LIGHT_THEME);
        let system = read_dword_as_light(hkey, SYSTEM_USES_LIGHT_THEME);
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        Ok(PersonalizeDwords {
            apps_use_light: apps,
            system_uses_light: system,
        })
    }

    pub fn try_write(values: PersonalizeDwords) -> Result<(), String> {
        let subkey = to_wide_null(PERSONALIZE_SUBKEY);
        let mut hkey = HKEY::default();
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                Some(0),
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("Unable to open HKCU Personalize key ({})", status.0));
        }

        let r1 = write_dword(hkey, APPS_USE_LIGHT_THEME, u32::from(values.apps_use_light));
        let r2 = write_dword(hkey, SYSTEM_USES_LIGHT_THEME, u32::from(values.system_uses_light));
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        r1?;
        r2?;
        Ok(())
    }
}

fn read_dword_as_light(hkey: HKEY, name: &str) -> bool {
    let wname = to_wide_null(name);
    let mut data: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(wname.as_ptr()),
            None,
            None,
            Some((&mut data as *mut u32).cast::<u8>()),
            Some(&mut size),
        )
    };
    if status == ERROR_SUCCESS && size >= 4 {
        data != 0
    } else {
        true
    }
}

fn write_dword(hkey: HKEY, name: &str, value: u32) -> Result<(), String> {
    let wname = to_wide_null(name);
    let bytes = value.to_le_bytes();
    let status = unsafe { RegSetValueExW(hkey, PCWSTR(wname.as_ptr()), Some(0), REG_DWORD, Some(&bytes)) };
    if status != ERROR_SUCCESS {
        Err(format!("RegSetValueExW({name}) failed ({})", status.0))
    } else {
        Ok(())
    }
}

pub struct RunKeyService;

impl RunKeyService {
    pub fn is_enabled() -> bool {
        let subkey = to_wide_null(RUN_SUBKEY);
        let mut hkey = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                Some(0),
                KEY_READ,
                &mut hkey,
            )
        };
        if status != ERROR_SUCCESS {
            return false;
        }
        let wname = to_wide_null(RUN_VALUE_NAME);
        let mut size = 0u32;
        let query = unsafe {
            RegQueryValueExW(
                hkey,
                PCWSTR(wname.as_ptr()),
                None,
                None,
                None,
                Some(&mut size),
            )
        };
        unsafe {
            let _ = RegCloseKey(hkey);
        }
        query == ERROR_SUCCESS && size > 2
    }

    pub fn try_set_enabled(enabled: bool, executable_path: &Path) -> Result<(), String> {
        let subkey = to_wide_null(RUN_SUBKEY);
        let mut hkey = HKEY::default();
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                Some(0),
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("Unable to open HKCU Run key ({})", status.0));
        }

        let wname = to_wide_null(RUN_VALUE_NAME);
        let res = if enabled {
            let path_str = executable_path.to_string_lossy();
            if path_str.trim().is_empty() || !executable_path.exists() {
                Err("Executable path is missing; cannot write the Run key.".to_string())
            } else {
                let quoted = if path_str.contains(' ') && !path_str.starts_with('"') {
                    format!("\"{path_str}\"")
                } else {
                    path_str.into_owned()
                };
                let wide = to_wide_null(&quoted);
                let byte_len = wide.len() * std::mem::size_of::<u16>();
                let bytes = unsafe { std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), byte_len) };
                let s = unsafe { RegSetValueExW(hkey, PCWSTR(wname.as_ptr()), Some(0), REG_SZ, Some(bytes)) };
                if s != ERROR_SUCCESS {
                    Err(format!("RegSetValueExW failed ({})", s.0))
                } else {
                    Ok(())
                }
            }
        } else {
            let s = unsafe { RegDeleteValueW(hkey, PCWSTR(wname.as_ptr())) };
            if s == ERROR_SUCCESS || s.0 == 2 {
                Ok(())
            } else {
                Err(format!("RegDeleteValueW failed ({})", s.0))
            }
        };

        unsafe {
            let _ = RegCloseKey(hkey);
        }
        res
    }
}
