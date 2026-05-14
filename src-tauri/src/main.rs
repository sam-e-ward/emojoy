// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use simplelog::*;
use std::fs::{self, OpenOptions};

fn main() {
    // Log to ~/Library/Logs/Emojoy/emojoy.log so we can read it even when
    // launched standalone (double-clicked), not just from the terminal.
    let log_dir = dirs::home_dir()
        .unwrap_or_default()
        .join("Library/Logs/Emojoy");
    let _ = fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("emojoy.log");

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open log file");

    let config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .build();

    // Write to both stderr (terminal) and the log file (standalone).
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, config.clone(), TerminalMode::Stderr, ColorChoice::Auto),
        WriteLogger::new(LevelFilter::Info, config, log_file),
    ])
    .expect("Failed to init logger");

    log::info!("=== Emojoy starting (release={}) ===", !cfg!(debug_assertions));
    log::info!("Log file: {}", log_path.display());
    emojoy_lib::run()
}
