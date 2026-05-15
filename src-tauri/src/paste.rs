use arboard::Clipboard;
use std::error::Error;

/// Simulate N backspace key presses to delete the trigger characters
pub fn delete_chars(count: usize) -> Result<(), Box<dyn Error>> {
    log::info!("[paste] deleting {} chars", count);

    #[cfg(target_os = "macos")]
    {
        use core_graphics::event::CGEvent;
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        const KEY_BACKSPACE: u16 = 51;

        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "Failed to create event source")?;

        for _ in 0..count {
            let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_BACKSPACE, true)
                .map_err(|_| "Failed to create key down event")?;
            key_down.post(core_graphics::event::CGEventTapLocation::HID);

            let key_up = CGEvent::new_keyboard_event(source.clone(), KEY_BACKSPACE, false)
                .map_err(|_| "Failed to create key up event")?;
            key_up.post(core_graphics::event::CGEventTapLocation::HID);

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    Ok(())
}

/// Copy an emoji to clipboard and simulate paste (Cmd+V on macOS, Ctrl+V on Windows)
pub fn paste_emoji(emoji: &str) -> Result<(), Box<dyn Error>> {
    log::info!("[paste] pasting {:?}", emoji);

    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(emoji)?;

    // Small delay to let clipboard settle
    std::thread::sleep(std::time::Duration::from_millis(50));

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

    const KEY_V: CGKeyCode = 9;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Failed to create event source")?;

    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_V, true)
        .map_err(|_| "Failed to create key down event")?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(core_graphics::event::CGEventTapLocation::HID);

    let key_up = CGEvent::new_keyboard_event(source, KEY_V, false)
        .map_err(|_| "Failed to create key up event")?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

#[cfg(target_os = "windows")]
fn simulate_paste_windows() -> Result<(), Box<dyn Error>> {
    // TODO: Implement using winapi SendInput
    Ok(())
}
