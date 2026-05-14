#![allow(deprecated)] // cocoa crate deprecation warnings (recommends objc2)

use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::Mutex;

static PREVIOUS_APP_PID: Mutex<Option<i32>> = Mutex::new(None);

/// Capture the currently focused app's PID (call before showing the popup)
pub fn capture_frontmost_app() {
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let front_app: id = msg_send![workspace, frontmostApplication];
        if front_app != nil {
            let pid: i32 = msg_send![front_app, processIdentifier];
            log::info!("[focus] captured frontmost app pid={}", pid);
            let mut stored = PREVIOUS_APP_PID.lock().unwrap();
            *stored = Some(pid);
        } else {
            log::warn!("[focus] frontmostApplication returned nil");
        }
    }
}

/// Reactivate the previously focused app (call before pasting)
pub fn reactivate_previous_app() {
    let pid = {
        let stored = PREVIOUS_APP_PID.lock().unwrap();
        *stored
    };

    if let Some(pid) = pid {
        log::info!("[focus] reactivating previous app pid={}", pid);
        unsafe {
            let running_app: id = msg_send![
                class!(NSRunningApplication),
                runningApplicationWithProcessIdentifier: pid
            ];
            if running_app != nil {
                // NSApplicationActivateIgnoringOtherApps = 1 << 1 = 2
                let result: bool = msg_send![
                    running_app,
                    activateWithOptions: 2u64
                ];
                log::info!("[focus] activateWithOptions result={}", result);
            } else {
                log::warn!("[focus] runningApplicationWithProcessIdentifier returned nil for pid={}", pid);
            }
        }
    } else {
        log::warn!("[focus] no previous app pid stored, nothing to reactivate");
    }
}
