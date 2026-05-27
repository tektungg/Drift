#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Re-export from the library crate
use drift as _;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
