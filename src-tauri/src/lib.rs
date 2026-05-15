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
    log::info!("[select] select_emoji called, emoji={:?}", emoji);

    // Record usage
    {
        let mut state = state.lock().unwrap();
        state.usage.record(&emoji);
        if let Err(e) = state.usage.save() {
            log::error!("[select] failed to save usage stats: {}", e);
        }
    }

    // Hide the popup first
    if let Some(window) = app.get_webview_window("main") {
        log::info!("[select] hiding window");
        let _ = window.hide();
    }

    // Reactivate the previously focused app
    #[cfg(target_os = "macos")]
    {
        log::info!("[select] reactivating previous app");
        focus_mac::reactivate_previous_app();
    }

    // Wait for the app to regain focus
    log::info!("[select] waiting 150ms for focus switch");
    std::thread::sleep(std::time::Duration::from_millis(150));

    // Paste the emoji
    log::info!("[select] pasting emoji");
    paste::paste_emoji(&emoji).map_err(|e| {
        log::error!("[select] paste_emoji failed: {}", e);
        e.to_string()
    })
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
    log::info!("[app] run() called");

    // Load config
    let config = AppConfig::load();
    log::info!("[app] config loaded, trigger={:?}", config.trigger_sequence);

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
                if !permissions_mac::is_accessibility_trusted(false) {
                    log::warn!("[app] Accessibility permission not granted — prompting user");
                    permissions_mac::is_accessibility_trusted(true); // shows System Settings

                    // Poll in background; once granted, restart so the kernel's
                    // TCC session cache is fresh for CGEventTapCreate.
                    let restart_handle = app.handle().clone();
                    std::thread::spawn(move || {
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            if permissions_mac::is_accessibility_trusted(false) {
                                log::info!("[app] Accessibility permission granted — restarting app");
                                let exe = std::env::current_exe().expect("can't find own exe");
                                log::info!("[app] re-launching: {:?}", exe);
                                let _ = std::process::Command::new(exe).spawn();
                                restart_handle.exit(0);
                                return;
                            }
                            log::info!("[app] Still waiting for accessibility permission...");
                        }
                    });
                } else {
                    log::info!("[app] Accessibility permission already granted");
                    let handle = app.handle().clone();
                    if !hotkey_mac::start_listener(handle, trigger.clone()) {
                        log::error!("[app] Event tap creation failed.");
                        log::error!("[app] Accessibility is granted but Input Monitoring may not be.");
                        log::error!("[app] Opening Input Monitoring settings — enable Emojoy, then restart the app.");
                        permissions_mac::open_input_monitoring_settings();
                    }
                }
            }

            // Listen for trigger event to show window
            let app_handle = app.handle().clone();
            app.listen("trigger-activated", move |_event| {
                log::info!("[trigger] trigger-activated received on main thread");

                // Capture which app is focused BEFORE we steal focus
                #[cfg(target_os = "macos")]
                {
                    log::info!("[trigger] capturing frontmost app");
                    focus_mac::capture_frontmost_app();
                }

                // Delete the trigger characters (e.g. "::") from the previous app
                log::info!("[trigger] deleting {} trigger chars", trigger_len);
                match paste::delete_chars(trigger_len) {
                    Ok(_) => log::info!("[trigger] delete_chars succeeded"),
                    Err(e) => log::error!("[trigger] delete_chars failed: {}", e),
                }

                log::info!("[trigger] waiting 50ms after delete");
                std::thread::sleep(std::time::Duration::from_millis(50));

                if let Some(window) = app_handle.get_webview_window("main") {
                    log::info!("[trigger] showing window");
                    let _ = window.center();
                    let _ = window.show();
                    let _ = window.set_focus();
                    log::info!("[trigger] window shown and focused");
                } else {
                    log::error!("[trigger] could not get main window!");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
