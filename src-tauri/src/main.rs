#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use drift::commands::{self, AppCtx};
use drift::engine::Engine;
use drift::events::TorrentDto;
use drift::settings::SettingsStore;
use drift::state::StateStore;
use std::sync::Arc;
use tauri::Emitter;

fn app_data_dir() -> std::path::PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("Drift")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let data_dir = app_data_dir();
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let state = Arc::new(StateStore::load_or_init(&data_dir).expect("load state"));
    let settings = Arc::new(SettingsStore::load_or_init(&data_dir).expect("load settings"));
    let engine = Engine::new(&data_dir.join("resume")).await.expect("init engine");

    // apply persisted limits
    let cfg0 = settings.get();
    engine.set_global_limits(cfg0.download_kbps, cfg0.upload_kbps);

    // Hold a clone of the engine for the progress-emit task; the ctx takes the other.
    let mut rx = engine.subscribe();
    let ctx = AppCtx { engine: engine.clone(), state: state.clone(), settings: settings.clone() };

    tauri::Builder::default()
        .manage(ctx)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::add_torrent,
            commands::pause,
            commands::resume,
            commands::remove,
            commands::peek,
            commands::get_settings,
            commands::set_settings,
            commands::open_folder,
            commands::copy_magnet,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            tokio::spawn(async move {
                while let Ok(u) = rx.recv().await {
                    let dto = TorrentDto {
                        infohash: u.infohash.as_str().into(),
                        name: String::new(), // resolved on the frontend via snapshot
                        downloaded: u.downloaded, total: u.total,
                        down_bps: u.down_bps, up_bps: u.up_bps,
                        peers: u.peers, state_label: u.state_label,
                    };
                    let _ = handle.emit("progress", dto);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
