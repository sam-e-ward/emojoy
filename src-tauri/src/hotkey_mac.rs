use std::ffi::c_void;
use std::sync::Mutex;
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

// ---- Raw FFI to CGEventTapCreate ----

type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFStringRef = *const c_void;
type CFAllocatorRef = *const c_void;
type CGEventRef = *mut c_void;

const K_CG_SESSION_EVENT_TAP: u32 = 1;
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const K_CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;
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

/// Context passed through CGEventTapCreate's userInfo pointer.
/// Heap-allocated and leaked — lives for the process lifetime.
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

    let mut buf = [0u16; 8];
    let mut len: i64 = 0;
    CGEventKeyboardGetUnicodeString(event as _, buf.len() as _, &mut len, buf.as_mut_ptr());

    if len > 0 {
        if let Ok(chars) = String::from_utf16(&buf[..len as usize]) {
            let mut state = ctx.state.lock().unwrap();
            for c in chars.chars() {
                if state.push_char(c) {
                    log::info!("[hotkey] trigger detected, emitting event");
                    let _ = ctx.app.emit("trigger-activated", ());
                }
            }
        }
    }

    std::ptr::null_mut() // listen-only, don't modify
}

/// Create a global keyboard event tap and schedule it on the main run loop.
///
/// Must be called from the main thread (inside Tauri's setup closure).
/// Requires both Accessibility and Input Monitoring permissions.
///
/// Returns true if the tap was created successfully.
pub fn start_listener(app_handle: AppHandle, trigger: String) -> bool {
    log::info!("[hotkey] starting listener for trigger={:?}", trigger);

    let ctx = Box::new(TapContext {
        state: Mutex::new(HotkeyState::new(&trigger)),
        app: app_handle,
    });
    let ctx_ptr = Box::into_raw(ctx) as *mut c_void;

    let event_mask: u64 = 1 << K_CG_EVENT_KEY_DOWN;

    let tap = unsafe {
        CGEventTapCreate(
            K_CG_SESSION_EVENT_TAP,
            K_CG_HEAD_INSERT_EVENT_TAP,
            K_CG_EVENT_TAP_OPTION_LISTEN_ONLY,
            event_mask,
            tap_callback,
            ctx_ptr,
        )
    };

    if tap.is_null() {
        log::error!("[hotkey] CGEventTapCreate failed — Input Monitoring permission likely missing");
        // Reclaim the leaked context
        unsafe { drop(Box::from_raw(ctx_ptr as *mut TapContext)); }
        return false;
    }

    unsafe {
        let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
        if source.is_null() {
            log::error!("[hotkey] CFMachPortCreateRunLoopSource failed");
            drop(Box::from_raw(ctx_ptr as *mut TapContext));
            return false;
        }

        let main_loop = CFRunLoopGetMain();
        let common_modes = core_foundation::runloop::kCFRunLoopCommonModes;
        CFRunLoopAddSource(main_loop, source, common_modes as *const _ as CFStringRef);
        CGEventTapEnable(tap, true);
    }

    log::info!("[hotkey] event tap active on main run loop");
    true
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
