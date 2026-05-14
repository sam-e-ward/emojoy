use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
};
use foreign_types_shared::ForeignType;
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

        // Check if buffer ends with trigger sequence
        if self.buffer.len() >= self.trigger_chars.len() {
            let start = self.buffer.len() - self.trigger_chars.len();
            if self.buffer[start..] == self.trigger_chars[..] {
                self.buffer.clear();
                return true;
            }
        }

        // Trim buffer to avoid unbounded growth
        if self.buffer.len() > 20 {
            self.buffer.drain(..10);
        }

        false
    }
}

/// Start listening for the trigger sequence globally.
/// Emits "trigger-activated" event to the Tauri app when detected.
pub fn start_listener(app_handle: AppHandle, trigger: String) {
    log::info!("[hotkey] start_listener called, trigger={:?}", trigger);

    std::thread::spawn(move || {
        log::info!("[hotkey] event tap thread started");
        let state = Arc::new(Mutex::new(HotkeyState::new(&trigger)));
        let app = app_handle.clone();

        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::KeyDown],
            move |_proxy, _event_type, event: &CGEvent| {
                // Get the characters from the event
                if let Some(chars) = get_event_characters(event) {
                    let mut state = state.lock().unwrap();
                    for c in chars.chars() {
                        if state.push_char(c) {
                            log::info!("[hotkey] trigger sequence detected! emitting trigger-activated");
                            match app.emit("trigger-activated", ()) {
                                Ok(_) => log::info!("[hotkey] trigger-activated emitted successfully"),
                                Err(e) => log::error!("[hotkey] failed to emit trigger-activated: {}", e),
                            }
                        }
                    }
                }
                None // Don't modify the event
            },
        );

        match tap {
            Ok(tap) => {
                log::info!("[hotkey] event tap created successfully, entering run loop");
                unsafe {
                    let loop_source = tap
                        .mach_port
                        .create_runloop_source(0)
                        .expect("Failed to create run loop source");
                    let run_loop = CFRunLoop::get_current();
                    run_loop.add_source(&loop_source, kCFRunLoopCommonModes);
                    tap.enable();
                    CFRunLoop::run_current();
                }
            }
            Err(_) => {
                log::error!("[hotkey] Failed to create event tap. Please grant Accessibility permission.");
                log::error!("[hotkey] System Preferences → Privacy & Security → Accessibility → Enable Emojoy");
            }
        }
    });
}

extern "C" {
    fn CGEventKeyboardGetUnicodeString(
        event: core_graphics::sys::CGEventRef,
        maxStringLength: core_foundation::base::CFIndex,
        actualStringLength: *mut core_foundation::base::CFIndex,
        unicodeString: *mut u16,
    );
}

/// Extract the character string from a CGEvent
fn get_event_characters(event: &CGEvent) -> Option<String> {
    let mut buf = [0u16; 8];
    let mut len: core_foundation::base::CFIndex = 0;

    unsafe {
        CGEventKeyboardGetUnicodeString(
            event.as_ptr(),
            buf.len() as _,
            &mut len,
            buf.as_mut_ptr(),
        );
    }

    if len > 0 {
        String::from_utf16(&buf[..len as usize]).ok()
    } else {
        None
    }
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
        // After match, buffer is cleared
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
