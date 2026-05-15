mod config;
mod emoji;
mod paste;
mod usage;

#[cfg(target_os = "macos")]
mod focus_mac;
#[cfg(target_os = "macos")]
mod hotkey_mac;
#[cfg(target_os = "macos")]
mod permissions_mac;

use config::AppConfig;
use emoji::EmojiDatabase;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Listener, Manager,
};
use usage::UsageStats;

struct AppState {
    db: EmojiDatabase,
    usage: UsageStats,
}

#[tauri::command]
fn search_emojis(
    query: &str,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Vec<emoji::SearchResult> {
    let state = state.lock().unwrap();
    state.db.search(query, 50, Some(&state.usage))
}

#[tauri::command]
fn select_emoji(
    emoji: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    log::info!("[select] emoji={:?}", emoji);

    // Record usage
    {
        let mut state = state.lock().unwrap();
        state.usage.record(&emoji);
        if let Err(e) = state.usage.save() {
            log::error!("[select] failed to save usage stats: {}", e);
        }
    }

    // Hide the popup
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    // Reactivate the previously focused app and paste
    #[cfg(target_os = "macos")]
    focus_mac::reactivate_previous_app();

    std::thread::sleep(std::time::Duration::from_millis(150));

    paste::paste_emoji(&emoji).map_err(|e| {
        log::error!("[select] paste failed: {}", e);
        e.to_string()
    })
}

#[tauri::command]
fn dismiss(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

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
    log::info!("[app] starting");

    let config = AppConfig::load();
    log::info!("[app] trigger={:?}", config.trigger_sequence);

    // Load emoji database
    let resource_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();

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
                Err(e) => log::error!("Failed to load emoji database from {:?}: {}", path, e),
            }
        }
    }

    let mut db = db.unwrap_or_else(|| {
        log::warn!("Could not load emoji database from any path");
        EmojiDatabase::from_entries(vec![])
    });

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

            #[cfg(target_os = "macos")]
            {
                // Accessibility is needed for posting synthetic keystrokes
                // (backspace to delete trigger chars, Cmd+V to paste emoji).
                if !permissions_mac::is_accessibility_trusted(false) {
                    log::warn!("[app] Accessibility permission not granted — prompting");
                    permissions_mac::is_accessibility_trusted(true);

                    let restart_handle = app.handle().clone();
                    std::thread::spawn(move || {
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            if permissions_mac::is_accessibility_trusted(false) {
                                log::info!("[app] Accessibility granted — restarting");
                                let exe = std::env::current_exe().expect("can't find own exe");
                                let _ = std::process::Command::new(exe).spawn();
                                restart_handle.exit(0);
                                return;
                            }
                        }
                    });
                } else {
                    // Input Monitoring is needed for the keyboard event tap.
                    // CGEventTapCreate returns NULL if this isn't granted.
                    let handle = app.handle().clone();
                    if !hotkey_mac::start_listener(handle, trigger.clone()) {
                        log::error!("[app] Event tap failed — opening Input Monitoring settings");
                        permissions_mac::open_input_monitoring_settings();
                    }
                }
            }

            // Listen for trigger event to show window
            let app_handle = app.handle().clone();
            app.listen("trigger-activated", move |_event| {
                log::info!("[trigger] activated");

                #[cfg(target_os = "macos")]
                focus_mac::capture_frontmost_app();

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
