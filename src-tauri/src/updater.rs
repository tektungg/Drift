//! In-app auto-updater flow.
//!
//! The actual scheme/endpoint live in `tauri.conf.json > plugins.updater`. This
//! module drives the UX: it asks the endpoint whether a newer **signed** build
//! exists, confirms with the user, then downloads and installs it while emitting
//! progress events the frontend renders as a toast.
//!
//! Two entry points share the logic:
//!   - launch check (`check(app, manual = false)`), spawned once on startup; and
//!   - the `check_for_updates` command, invoked by the "Check for updates" button
//!     in About (`manual = true`).
//!
//! All failures are logged and swallowed so a missing/unreachable `latest.json`
//! (e.g. no release published yet) never blocks startup. Manual checks also emit
//! `update-none` / `update-error` so the UI can give feedback.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

/// Set when the user dismisses an available update with "Later". Auto-checks then
/// stay quiet for the rest of the session (no nagging on every window focus /
/// relaunch within the same process), but an explicit manual check still runs.
static DEFERRED_THIS_SESSION: AtomicBool = AtomicBool::new(false);

/// Command behind the "Check for updates" button. Spawns the check so the
/// frontend `invoke` returns immediately; the UI is driven by the emitted events.
#[tauri::command]
pub fn check_for_updates(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        check(app, true).await;
    });
}

/// Core update flow. `manual` distinguishes a user-triggered check (which gives
/// explicit "up to date" / error feedback and ignores the session-defer flag)
/// from the silent launch check.
pub async fn check(app: AppHandle, manual: bool) {
    use tauri_plugin_updater::UpdaterExt;

    if !manual && DEFERRED_THIS_SESSION.load(Ordering::Relaxed) {
        return;
    }

    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("updater unavailable: {e}");
            if manual {
                let _ = app.emit("update-error", e.to_string());
            }
            return;
        }
    };

    let update = match updater.check().await {
        Ok(Some(u)) => u,
        Ok(None) => {
            tracing::info!("Drift is up to date");
            if manual {
                let _ = app.emit("update-none", ());
            }
            return;
        }
        Err(e) => {
            tracing::warn!("update check failed: {e}");
            if manual {
                let _ = app.emit("update-error", e.to_string());
            }
            return;
        }
    };

    // Confirm before touching the user's install.
    let accept = {
        use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
        let notes = update
            .body
            .clone()
            .filter(|b| !b.trim().is_empty())
            .map(|b| format!("\n\n{b}"))
            .unwrap_or_default();
        app.dialog()
            .message(format!(
                "Drift {} is available (you have {}).{}\n\nDownload and install it now?",
                update.version, update.current_version, notes
            ))
            .title("Update available")
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Install".into(),
                "Later".into(),
            ))
            .blocking_show()
    };
    if !accept {
        // Stay quiet for the rest of this session; the manual button still works.
        DEFERRED_THIS_SESSION.store(true, Ordering::Relaxed);
        return;
    }

    // Download + install, streaming progress to the frontend toast.
    let _ = app.emit("update-download-started", update.version.clone());
    let mut downloaded: u64 = 0;
    let progress_app = app.clone();
    let result = update
        .download_and_install(
            move |chunk_len, content_len| {
                downloaded += chunk_len as u64;
                let _ = progress_app.emit(
                    "update-download-progress",
                    serde_json::json!({ "downloaded": downloaded, "total": content_len }),
                );
            },
            || {},
        )
        .await;

    if let Err(e) = result {
        tracing::error!("update download/install failed: {e}");
        let _ = app.emit("update-error", e.to_string());
        return;
    }

    tracing::info!("update installed; restarting");
    let _ = app.emit("update-ready", ());
    app.restart();
}
