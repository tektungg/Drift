#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use drift::commands::{self, AppCtx};
use drift::engine::Engine;
use drift::events::TorrentDto;
use drift::settings::SettingsStore;
use drift::state::StateStore;
use std::sync::Arc;
use tauri::{Emitter, Manager};

fn app_data_dir() -> std::path::PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("Drift")
}

fn main() {
    tracing_subscriber::fmt::init();

    // IMPORTANT: the single-instance plugin must be registered FIRST and all the
    // heavy initialization (the librqbit engine, which binds DHT/peer sockets)
    // must happen inside `.setup()`. When a second copy launches — e.g. a magnet
    // clicked in a browser while Drift is already running — the single-instance
    // plugin detects the primary instance during `.run()`, forwards argv to it,
    // and terminates the secondary process BEFORE `.setup()` runs. If we created
    // the engine in `main()` (before the builder), the secondary would panic
    // trying to bind a port already held by the primary, the callback would never
    // fire, and the magnet would be silently dropped.
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // A second launch lands here on the PRIMARY instance. Stash the source
            // FIRST, then bring the window forward — the frontend re-pulls the
            // pending source on focus, which is reliable even when a cross-context
            // emit isn't delivered.
            let mut source: Option<String> = None;
            for arg in argv.iter().skip(1) {
                if arg.starts_with("magnet:?") || arg.ends_with(".torrent") {
                    commands::set_pending_source(arg.clone());
                    source = Some(arg.clone());
                    break;
                }
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
                // Emit on the window itself (more reliable than the app-wide emit
                // from this plugin-callback context). The frontend also pulls the
                // pending source on focus as a fallback.
                if let Some(src) = source {
                    let _ = w.emit("open-source", src);
                }
            }
        }))
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
            commands::torrent_files,
            commands::set_file_selection,
            commands::focus_main,
            commands::pick_folder,
            commands::pick_torrent_file,
            commands::take_pending_source,
        ])
        .setup(|app| {
            // ── Heavy init (runs only on the PRIMARY instance) ──────────────────
            let data_dir = app_data_dir();
            std::fs::create_dir_all(&data_dir).expect("create data dir");
            let state = Arc::new(StateStore::load_or_init(&data_dir).expect("load state"));
            let settings = Arc::new(SettingsStore::load_or_init(&data_dir).expect("load settings"));
            let engine = tauri::async_runtime::block_on(Engine::new(&data_dir.join("resume")))
                .expect("init engine");

            // apply persisted limits
            let cfg0 = settings.get();
            engine.set_global_limits(cfg0.download_kbps, cfg0.upload_kbps);

            // Resume previously persisted torrents (non-paused ones)
            let snap = state.snapshot();
            tauri::async_runtime::block_on(async {
                for r in &snap.torrents {
                    if matches!(r.state, drift::state::TorrentState::Paused) { continue; }
                    if let Err(e) = engine
                        .resume_existing(
                            &drift::magnet::InfoHash(r.infohash.clone()),
                            &r.save_path,
                        )
                        .await
                    {
                        tracing::warn!("failed to resume {}: {e}", r.infohash);
                    }
                }
            });

            // Honor the queue cap on launch instead of blindly running everything.
            tauri::async_runtime::block_on(
                drift::queue::reconcile(&engine, &state, cfg0.max_active_downloads)
            );

            // Hold a clone of the engine for the progress-emit task; the ctx takes
            // the other.
            let mut rx = engine.subscribe();
            let ctx = AppCtx {
                engine: engine.clone(),
                state: state.clone(),
                settings: settings.clone(),
            };
            app.manage(ctx);

            let handle = app.handle().clone();
            let state_for_emit = state.clone();
            let engine_for_emit = engine.clone();
            let settings_for_emit = settings.clone();
            tauri::async_runtime::spawn(async move {
                use drift::state::TorrentState;
                use std::collections::HashMap;
                let mut last_state_label: HashMap<String, String> = HashMap::new();
                while let Ok(u) = rx.recv().await {
                    let ih_str: String = u.infohash.as_str().into();

                    // Look up display name and current persisted record from state.
                    let snap = state_for_emit.snapshot();
                    let rec_opt = snap.torrents.iter().find(|t| t.infohash == ih_str).cloned();
                    let name = rec_opt.as_ref().map(|r| r.display_name.clone()).unwrap_or_default();
                    let added_at = rec_opt.as_ref().map(|r| r.added_at).unwrap_or(0);

                    // Persist state transitions so the sidebar counts reflect reality across restarts.
                    let prev_emitted = last_state_label.get(&ih_str).cloned();
                    if prev_emitted.as_deref() != Some(u.state_label.as_str()) {
                        last_state_label.insert(ih_str.clone(), u.state_label.clone());
                        if let Some(mut rec) = rec_opt {
                            let new_state = match u.state_label.as_str() {
                                "downloading" => Some(TorrentState::Downloading),
                                "seeding"     => Some(TorrentState::Seeding),
                                // Engine reports "paused" for BOTH user-paused and
                                // queued torrents. Preserve whichever the record
                                // already holds; never downgrade Queued -> Paused.
                                "paused" => Some(match rec.state {
                                    TorrentState::Queued => TorrentState::Queued,
                                    _ => TorrentState::Paused,
                                }),
                                "stalled"     => Some(TorrentState::Stalled),
                                "completed"   => Some(TorrentState::Completed),
                                _             => None, // initializing/error/etc. — don't persist
                            };
                            if let Some(s) = new_state {
                                if rec.state != s {
                                    rec.state = s;
                                    let _ = state_for_emit.upsert(rec);
                                }
                            }
                            // A finished download frees a slot — let the queue advance.
                            if matches!(u.state_label.as_str(), "seeding" | "completed") {
                                drift::queue::reconcile(
                                    &engine_for_emit, &state_for_emit,
                                    settings_for_emit.get().max_active_downloads,
                                ).await;
                            }
                        }
                    }

                    let dto = TorrentDto {
                        infohash: ih_str,
                        name,
                        downloaded: u.downloaded, total: u.total, uploaded: u.uploaded,
                        down_bps: u.down_bps, up_bps: u.up_bps,
                        peers: u.peers, added_at,
                        state_label: u.state_label,
                    };
                    let _ = handle.emit("progress", dto);
                }
            });

            // Window events: close-to-tray + native drag-drop handling.
            //
            // Drag-drop NOTE: Tauri 2 emits `tauri://drag-drop` only to its own
            // window-labeled event target. The CDN-loaded @tauri-apps/api/event
            // `listen()` does not always pick that up reliably across minor
            // versions. Handling it in Rust via WindowEvent::DragDrop is the
            // stable path — we then re-emit the file path through our existing
            // `open-source` event which the frontend already subscribes to.
            let main_window = app.get_webview_window("main").unwrap();
            let settings_for_close = settings.clone();
            let app_handle_for_close = app.handle().clone();
            main_window.on_window_event(move |event| {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        if settings_for_close.get().close_to_tray {
                            api.prevent_close();
                            if let Some(w) = app_handle_for_close.get_webview_window("main") {
                                let _ = w.hide();
                            }
                        }
                    }
                    tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                        // Prefer .torrent files; otherwise take the first dropped path
                        // (which is fine if the user drops a single .torrent without
                        // the extension preserved, or a magnet shortcut, etc.).
                        let chosen = paths
                            .iter()
                            .find(|p| p.extension().map_or(false, |e| e.eq_ignore_ascii_case("torrent")))
                            .or_else(|| paths.first());
                        if let Some(path) = chosen {
                            let payload = path.to_string_lossy().into_owned();
                            let _ = app_handle_for_close.emit("open-source", payload);
                        }
                    }
                    _ => {}
                }
            });

            // Install system tray
            drift::tray::install(app.handle())?;

            // Start the clipboard watcher
            drift::clipboard::start(app.handle().clone());

            // Position the magnet-toast bottom-right of the primary monitor
            if let Some(t) = app.get_webview_window("magnet-toast") {
                if let Ok(Some(m)) = t.primary_monitor() {
                    let size = m.size();
                    let _ = t.set_position(tauri::PhysicalPosition::new(
                        size.width as i32 - 360 - 16,
                        size.height as i32 - 120 - 56, // 56 leaves room above taskbar
                    ));
                }
            }

            // Cold-start: if launched with a magnet/torrent arg, stash it for the
            // frontend to pull once its listeners are ready (avoids the race where
            // a one-shot event fires before the webview registers a handler).
            let argv: Vec<String> = std::env::args().collect();
            for arg in argv.iter().skip(1) {
                if arg.starts_with("magnet:?") || arg.ends_with(".torrent") {
                    commands::set_pending_source(arg.clone());
                    break;
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
