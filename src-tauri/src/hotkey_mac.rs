use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::CGEvent;
use foreign_types_shared::ForeignType;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// Maximum time between keystrokes to count as a sequence (ms)
const SEQUENCE_TIMEOUT_MS: u128 = 500;

struct HotkeyState {
    trigger_chars: Vec<char>,
    buffer: Vec<char>,
    last_keystroke: Instant,
}

impl HotkeyState {
    fn new(trigger: &str) -> Self {
        HotkeyState {
            trigger_chars: trigger.chars().collect(),
            buffer: Vec::new(),
            last_keystroke: Instant::now(),
        }
    }

    fn push_char(&mut self, c: char) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_keystroke).as_millis() > SEQUENCE_TIMEOUT_MS {
            self.buffer.clear();
        }
        self.last_keystroke = now;

        self.buffer.push(c);

        if self.buffer.len() >= self.trigger_chars.len() {
            let start = self.buffer.len() - self.trigger_chars.len();
            if self.buffer[start..] == self.trigger_chars[..] {
                self.buffer.clear();
                return true;
            }
        }

        if self.buffer.len() > 20 {
            self.buffer.drain(..10);
        }

        false
    }
}

// ---- Raw FFI to CGEventTapCreate (bypass core-graphics crate wrapper) ----

type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFStringRef = *const c_void;
type CFAllocatorRef = *const c_void;
type CGEventRef = *mut c_void;

// CGEventTapLocation
const K_CG_SESSION_EVENT_TAP: u32 = 1;
const K_CG_HID_EVENT_TAP: u32 = 0;
// CGEventTapPlacement
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
// CGEventTapOptions
const K_CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;
// CGEventType::KeyDown
const K_CG_EVENT_KEY_DOWN: u64 = 10;

type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

    fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: i64,
    ) -> CFRunLoopSourceRef;

    fn CFRunLoopGetMain() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);

    fn CGEventKeyboardGetUnicodeString(
        event: core_graphics::sys::CGEventRef,
        max_len: i64,
        actual_len: *mut i64,
        buf: *mut u16,
    );
}

/// Context passed through CGEventTapCreate's userInfo pointer
struct TapContext {
    state: Mutex<HotkeyState>,
    app: AppHandle,
}

unsafe extern "C" fn tap_callback(
    _proxy: *mut c_void,
    _event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    let ctx = &*(user_info as *const TapContext);

    // Extract characters from the event
    let mut buf = [0u16; 8];
    let mut len: i64 = 0;
    CGEventKeyboardGetUnicodeString(event as _, buf.len() as _, &mut len, buf.as_mut_ptr());

    if len > 0 {
        if let Ok(chars) = String::from_utf16(&buf[..len as usize]) {
            let mut state = ctx.state.lock().unwrap();
            for c in chars.chars() {
                if state.push_char(c) {
                    log::info!("[hotkey] trigger sequence detected! emitting trigger-activated");
                    match ctx.app.emit("trigger-activated", ()) {
                        Ok(_) => log::info!("[hotkey] trigger-activated emitted successfully"),
                        Err(e) => log::error!("[hotkey] failed to emit trigger-activated: {}", e),
                    }
                }
            }
        }
    }

    std::ptr::null_mut() // don't modify the event (listen-only)
}

/// Start listening for the trigger sequence globally.
///
/// Creates the event tap and schedules it on the **main thread's** run loop.
/// Must be called from the main thread (i.e., inside Tauri's setup closure).
///
/// Returns true if the tap was created successfully.
pub fn start_listener(app_handle: AppHandle, trigger: String) -> bool {
    log::info!("[hotkey] start_listener called, trigger={:?}", trigger);

    let event_mask: u64 = 1 << K_CG_EVENT_KEY_DOWN;

    // Heap-allocate the context and leak it — it must live for the process lifetime
    let ctx = Box::new(TapContext {
        state: Mutex::new(HotkeyState::new(&trigger)),
        app: app_handle,
    });
    let ctx_ptr = Box::into_raw(ctx) as *mut c_void;

    // Try Session tap first, then HID tap as fallback
    let locations = [
        (K_CG_SESSION_EVENT_TAP, "Session"),
        (K_CG_HID_EVENT_TAP, "HID"),
    ];

    for (location, name) in &locations {
        log::info!("[hotkey] trying CGEventTapCreate with location={}", name);

        let tap = unsafe {
            CGEventTapCreate(
                *location,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_LISTEN_ONLY,
                event_mask,
                tap_callback,
                ctx_ptr,
            )
        };

        if tap.is_null() {
            log::warn!("[hotkey] CGEventTapCreate returned NULL for location={}", name);
            continue;
        }

        log::info!("[hotkey] event tap created successfully (location={})", name);

        unsafe {
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                log::error!("[hotkey] CFMachPortCreateRunLoopSource returned NULL");
                continue;
            }

            // Schedule on the MAIN run loop (not a background thread)
            let main_loop = CFRunLoopGetMain();
            // kCFRunLoopCommonModes
            let common_modes = core_foundation::runloop::kCFRunLoopCommonModes;
            CFRunLoopAddSource(main_loop, source, common_modes as *const _ as CFStringRef);
            CGEventTapEnable(tap, true);
        }

        log::info!("[hotkey] event tap scheduled on main run loop");
        return true;
    }

    // If we get here, all locations failed — reclaim the context
    unsafe {
        drop(Box::from_raw(ctx_ptr as *mut TapContext));
    }

    log::error!("[hotkey] Failed to create event tap with any location");
    log::error!("[hotkey] Please grant Accessibility permission in System Preferences → Privacy & Security → Accessibility");
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_detection() {
        let mut state = HotkeyState::new("::");
        assert!(!state.push_char('h'));
        assert!(!state.push_char('e'));
        assert!(!state.push_char(':'));
        assert!(state.push_char(':'));
    }

    #[test]
    fn test_trigger_resets_after_match() {
        let mut state = HotkeyState::new("::");
        assert!(!state.push_char(':'));
        assert!(state.push_char(':'));
        assert!(!state.push_char(':'));
        assert!(state.push_char(':'));
    }

    #[test]
    fn test_no_false_trigger() {
        let mut state = HotkeyState::new("::");
        assert!(!state.push_char(':'));
        assert!(!state.push_char('a'));
        assert!(!state.push_char(':'));
        assert!(state.push_char(':'));
    }
}
