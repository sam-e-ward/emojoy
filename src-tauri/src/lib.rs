mod config;
mod emoji;
mod paste;
mod usage;

#[cfg(target_os = "macos")]
mod focus_mac;
#[cfg(target_os = "macos")]
mod hotkey_mac;

use emoji::EmojiDatabase;
use config::AppConfig;
use usage::UsageStats;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Listener, Manager,
};

struct AppState {
    db: EmojiDatabase,
    usage: UsageStats,
}

#[tauri::command]
fn search_emojis(query: &str, state: tauri::State<'_, Mutex<AppState>>) -> Vec<emoji::SearchResult> {
    let state = state.lock().unwrap();
    state.db.search(query, 50, Some(&state.usage))
}

#[tauri::command]
fn select_emoji(emoji: String, app: tauri::AppHandle, state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    // Record usage
    {
        let mut state = state.lock().unwrap();
        state.usage.record(&emoji);
        if let Err(e) = state.usage.save() {
            eprintln!("Failed to save usage stats: {}", e);
        }
    }

    // Hide the popup first
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    // Reactivate the previously focused app
    #[cfg(target_os = "macos")]
    focus_mac::reactivate_previous_app();

    // Wait for the app to regain focus
    std::thread::sleep(std::time::Duration::from_millis(150));

    // Paste the emoji
    paste::paste_emoji(&emoji).map_err(|e| e.to_string())
}

#[tauri::command]
fn dismiss(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    // Refocus the previous app
    #[cfg(target_os = "macos")]
    focus_mac::reactivate_previous_app();
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let quit = MenuItem::with_id(app, "quit", "Quit Emojoy", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .icon_as_template(false)
        .menu(&menu)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load config
    let config = AppConfig::load();

    // Load emoji database
    let resource_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

    // Try multiple paths for the emoji data
    let emoji_paths = vec![
        resource_path.join("../Resources/resources/emoji-en-US.json"), // macOS bundle
        resource_path.join("resources/emoji-en-US.json"),              // dev / Windows
        std::path::PathBuf::from("src-tauri/resources/emoji-en-US.json"), // cargo test
    ];

    let mut db = None;
    for path in &emoji_paths {
        if path.exists() {
            match EmojiDatabase::load(path) {
                Ok(database) => {
                    db = Some(database);
                    break;
                }
                Err(e) => eprintln!("Failed to load emoji database from {:?}: {}", path, e),
            }
        }
    }

    let mut db = db.unwrap_or_else(|| {
        eprintln!("Warning: Could not load emoji database from any path");
        EmojiDatabase::from_entries(vec![])
    });

    // Merge custom aliases
    if !config.custom_aliases.is_empty() {
        db.merge_aliases(&config.custom_aliases);
    }

    let usage = UsageStats::load();
    let trigger = config.trigger_sequence.clone();
    let trigger_len = trigger.chars().count();

    tauri::Builder::default()
        .manage(Mutex::new(AppState { db, usage }))
        .invoke_handler(tauri::generate_handler![
            search_emojis,
            select_emoji,
            dismiss,
        ])
        .setup(move |app| {
            setup_tray(app)?;

            // Start global keystroke listener (macOS)
            #[cfg(target_os = "macos")]
            {
                let handle = app.handle().clone();
                hotkey_mac::start_listener(handle, trigger);
            }

            // Listen for trigger event to show window
            let app_handle = app.handle().clone();
            app.listen("trigger-activated", move |_event| {
                // Capture which app is focused BEFORE we steal focus
                #[cfg(target_os = "macos")]
                focus_mac::capture_frontmost_app();

                // Delete the trigger characters (e.g. "::") from the previous app
                let _ = paste::delete_chars(trigger_len);
                std::thread::sleep(std::time::Duration::from_millis(50));

                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.center();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
