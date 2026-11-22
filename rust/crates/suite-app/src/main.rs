#![windows_subsystem = "windows"]

mod capture_ui;
mod color_pick_ui;
mod controller;
mod d2d;
mod history_ui;
mod hotkeys;
mod messages;
mod netspeed_ui;
mod pin_ui;
mod scroll_ui;
mod settings_ui;
mod tray;
mod update_checker;
mod win32_util;

use std::io::Write;

use suite_platform::SingleInstanceGuard;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, TranslateMessage,
};

use crate::controller::AppController;

fn main() {
    let smoke_check = std::env::args().any(|a| a == "--smoke-check");
    if smoke_check {
        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    let Some(_instance_guard) = SingleInstanceGuard::try_acquire() else {
        if smoke_check {
            eprintln!("{{\"error\":\"SingleInstanceGuard already held\"}}");
            std::process::exit(2);
        }
        return;
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let controller = match AppController::start(smoke_check) {
        Ok(c) => c,
        Err(err) => {
            if smoke_check {
                eprintln!("{{\"error\":{}}}", serde_json::json!(err));
                unsafe {
                    CoUninitialize();
                }
                std::process::exit(1);
            }
            unsafe {
                CoUninitialize();
            }
            return;
        }
    };

    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    if smoke_check
        && let Some(report) = controller.take_smoke_report()
            && let Ok(json) = serde_json::to_string(&report)
        {
            println!("{json}");
            let _ = std::io::stdout().flush();
        }

    drop(controller);
    unsafe {
        CoUninitialize();
    }
}
