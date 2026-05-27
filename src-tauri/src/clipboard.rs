use crate::commands::AppCtx;
use crate::magnet;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};

pub static ENABLED: AtomicBool = AtomicBool::new(true);

/// Background thread that polls the Windows clipboard every 500ms looking
/// for newly-copied magnet links. When it sees one that's NOT already in
/// the user's torrent list, it emits `magnet-detected` so the toast window
/// can offer to add it.
///
/// We compare clipboard TEXT (not Windows' opaque sequence number) because:
///   1. the sequence-number API can misbehave in some sandbox / privilege
///      contexts and silently report "no change" forever;
///   2. text comparison is naturally idempotent — copying the same magnet
///      twice in a row only fires once, and copying B then re-copying A
///      correctly fires for A again because the text changed.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_text: Option<String> = None;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if !ENABLED.load(Ordering::Relaxed) {
                continue;
            }

            let Ok(txt) = clipboard_win::get_clipboard_string() else {
                continue;
            };
            if last_text.as_deref() == Some(txt.as_str()) {
                continue;
            }
            last_text = Some(txt.clone());

            if !txt.starts_with("magnet:?") {
                continue;
            }
            let Ok(parsed) = magnet::parse(&txt) else {
                continue;
            };
            // Skip if already in the user's torrent list — don't nag.
            let ctx = app.state::<AppCtx>();
            if ctx.state.contains(parsed.infohash.as_str()) {
                continue;
            }
            let _ = app.emit(
                "magnet-detected",
                serde_json::json!({
                    "infohash": parsed.infohash.as_str(),
                    "name": parsed.display_name.clone().unwrap_or_else(|| parsed.infohash.as_str().to_string()),
                    "uri": txt,
                }),
            );
        }
    });
}
