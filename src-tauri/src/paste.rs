use arboard::Clipboard;
use std::error::Error;

/// Copy an emoji to clipboard and simulate paste (Cmd+V on macOS, Ctrl+V on Windows)
pub fn paste_emoji(emoji: &str) -> Result<(), Box<dyn Error>> {
    // Write to clipboard
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(emoji)?;

    // Small delay to let clipboard settle
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Simulate paste keystroke
    #[cfg(target_os = "macos")]
    simulate_paste_macos()?;

    #[cfg(target_os = "windows")]
    simulate_paste_windows()?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn simulate_paste_macos() -> Result<(), Box<dyn Error>> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    // Key code for 'V' on macOS
    const KEY_V: CGKeyCode = 9;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Failed to create event source")?;

    // Key down
    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_V, true)
        .map_err(|_| "Failed to create key down event")?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(core_graphics::event::CGEventTapLocation::HID);

    // Key up
    let key_up = CGEvent::new_keyboard_event(source, KEY_V, false)
        .map_err(|_| "Failed to create key up event")?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

#[cfg(target_os = "windows")]
fn simulate_paste_windows() -> Result<(), Box<dyn Error>> {
    // TODO: Implement using winapi SendInput
    // use winapi::um::winuser::{SendInput, INPUT, INPUT_KEYBOARD, ...};
    Ok(())
}
