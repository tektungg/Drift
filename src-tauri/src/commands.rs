use crate::engine::{Engine, Source};
use crate::events::{AddRequest, TorrentDto};
use crate::magnet::InfoHash;
use crate::settings::SettingsStore;
use crate::state::{StateStore, TorrentRecord, TorrentState};
use std::path::PathBuf;
use std::sync::Arc;

pub struct AppCtx {
    pub engine: Engine,
    pub state: Arc<StateStore>,
    pub settings: Arc<SettingsStore>,
}

#[tauri::command]
pub async fn snapshot(ctx: tauri::State<'_, AppCtx>) -> Result<Vec<TorrentDto>, String> {
    let snap = ctx.state.snapshot();
    Ok(snap.torrents.into_iter().map(|r| TorrentDto {
        infohash: r.infohash, name: r.display_name,
        downloaded: 0, total: r.total_size, down_bps: 0, up_bps: 0, peers: 0,
        state_label: match r.state {
            TorrentState::Downloading => "downloading",
            TorrentState::Seeding => "seeding",
            TorrentState::Paused => "paused",
            TorrentState::Completed => "completed",
            TorrentState::Stalled => "stalled",
        }.into(),
    }).collect())
}

#[tauri::command]
pub async fn add_torrent(ctx: tauri::State<'_, AppCtx>, req: AddRequest) -> Result<String, String> {
    let src = if req.source.starts_with("magnet:?") {
        Source::Magnet(req.source.clone())
    } else { Source::TorrentFile(PathBuf::from(&req.source)) };

    let meta = ctx.engine.peek(&src).await.map_err(|e| e.to_string())?;
    if ctx.state.contains(meta.infohash.as_str()) {
        // Clean up the list-only registration that peek leaves in the librqbit session.
        let _ = ctx.engine.remove(&meta.infohash, false).await;
        return Err("already_added".into());
    }

    let cfg = ctx.settings.get();
    let save_path = match req.override_path {
        Some(p) => PathBuf::from(p),
        None => Engine::pick_save_path(&cfg.download_root, &meta, &cfg.category_map.clone().into()),
    };
    std::fs::create_dir_all(&save_path).map_err(|e| e.to_string())?;

    // peek added the torrent in list-only mode. librqbit returns AlreadyManaged on the
    // subsequent start() with the real options (output folder, file selection), so the
    // download never actually begins. Forget the list-only registration first.
    let _ = ctx.engine.remove(&meta.infohash, false).await;

    let ih = ctx.engine.start(src, &save_path, req.selected_files.clone()).await.map_err(|e| e.to_string())?;

    ctx.state.upsert(TorrentRecord {
        infohash: ih.as_str().into(),
        display_name: meta.name.clone(),
        save_path,
        state: TorrentState::Downloading,
        added_at: chrono_now_ms(),
        total_size: meta.total_size,
        selected_files: req.selected_files,
    }).map_err(|e| e.to_string())?;

    Ok(ih.as_str().into())
}

#[tauri::command]
pub async fn pause(ctx: tauri::State<'_, AppCtx>, infohash: String) -> Result<(), String> {
    ctx.engine.pause(&InfoHash(infohash.clone())).await.map_err(|e| e.to_string())?;
    if let Some(mut r) = ctx.state.snapshot().torrents.into_iter().find(|t| t.infohash == infohash) {
        r.state = TorrentState::Paused;
        ctx.state.upsert(r).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn resume(ctx: tauri::State<'_, AppCtx>, infohash: String) -> Result<(), String> {
    ctx.engine.resume(&InfoHash(infohash.clone())).await.map_err(|e| e.to_string())?;
    if let Some(mut r) = ctx.state.snapshot().torrents.into_iter().find(|t| t.infohash == infohash) {
        r.state = TorrentState::Downloading;
        ctx.state.upsert(r).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn remove(ctx: tauri::State<'_, AppCtx>, infohash: String, delete_files: bool) -> Result<(), String> {
    ctx.engine.remove(&InfoHash(infohash.clone()), delete_files).await.map_err(|e| e.to_string())?;
    ctx.state.remove(&infohash).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn peek(ctx: tauri::State<'_, AppCtx>, source: String) -> Result<serde_json::Value, String> {
    let src = if source.starts_with("magnet:?") {
        Source::Magnet(source.clone())
    } else { Source::TorrentFile(PathBuf::from(&source)) };
    let m = ctx.engine.peek(&src).await.map_err(|e| e.to_string())?;
    let cfg = ctx.settings.get();
    let predicted = Engine::pick_save_path(&cfg.download_root, &m, &cfg.category_map.clone().into());
    Ok(serde_json::json!({
        "infohash": m.infohash.as_str(),
        "name": m.name,
        "total_size": m.total_size,
        "files": m.files.iter().map(|f| serde_json::json!({"path": f.path, "size": f.size})).collect::<Vec<_>>(),
        "predicted_save_path": predicted,
    }))
}

#[tauri::command]
pub fn get_settings(ctx: tauri::State<'_, AppCtx>) -> serde_json::Value {
    serde_json::to_value(ctx.settings.get()).unwrap()
}

#[tauri::command]
pub fn set_settings(ctx: tauri::State<'_, AppCtx>, value: serde_json::Value) -> Result<(), String> {
    let cfg: crate::settings::Config = serde_json::from_value(value).map_err(|e| e.to_string())?;
    ctx.engine.set_global_limits(cfg.download_kbps, cfg.upload_kbps);
    crate::clipboard::ENABLED.store(cfg.clipboard_watch, std::sync::atomic::Ordering::Relaxed);
    apply_startup_registration(cfg.start_with_windows).map_err(|e| e.to_string())?;
    ctx.settings.replace(cfg).map_err(|e| e.to_string())
}

fn apply_startup_registration(enable: bool) -> anyhow::Result<()> {
    use std::process::Command;
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    if enable {
        Command::new("reg").args(["add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v", "Drift", "/t", "REG_SZ", "/d", &exe, "/f"]).status()?;
    } else {
        let _ = Command::new("reg").args(["delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v", "Drift", "/f"]).status();
    }
    Ok(())
}

fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[tauri::command]
pub fn open_folder(ctx: tauri::State<'_, AppCtx>, infohash: String) -> Result<(), String> {
    let snap = ctx.state.snapshot();
    let rec = snap.torrents.iter().find(|t| t.infohash == infohash).ok_or("not_found")?;
    tauri_plugin_opener::open_path(&rec.save_path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_magnet(ctx: tauri::State<'_, AppCtx>, infohash: String) -> Result<(), String> {
    let snap = ctx.state.snapshot();
    let rec = snap.torrents.iter().find(|t| t.infohash == infohash).ok_or("not_found")?;
    let magnet = format!(
        "magnet:?xt=urn:btih:{}&dn={}",
        rec.infohash, urlencoding::encode(&rec.display_name)
    );
    clipboard_win::set_clipboard_string(&magnet).map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub async fn torrent_files(ctx: tauri::State<'_, AppCtx>, infohash: String) -> Result<serde_json::Value, String> {
    let files = ctx.engine.files(&crate::magnet::InfoHash(infohash)).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(files).unwrap())
}

#[tauri::command]
pub async fn set_file_selection(ctx: tauri::State<'_, AppCtx>, infohash: String, selected: Vec<usize>) -> Result<(), String> {
    ctx.engine.set_file_selection(&crate::magnet::InfoHash(infohash.clone()), &selected).await.map_err(|e| e.to_string())?;
    if let Some(mut r) = ctx.state.snapshot().torrents.into_iter().find(|t| t.infohash == infohash) {
        r.selected_files = Some(selected);
        ctx.state.upsert(r).map_err(|e| e.to_string())?;
    }
    Ok(())
}
