use crate::commands::AppCtx;
use crate::magnet;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};

pub static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_seq: u32 = 0;
        let mut dismissed: std::collections::HashSet<String> = Default::default();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if !ENABLED.load(Ordering::Relaxed) {
                continue;
            }

            // Use clipboard sequence number to detect changes (no-op open needed).
            let seq: u32 = clipboard_win::raw::seq_num()
                .map(|n| u32::from(n))
                .unwrap_or_else(|| last_seq.wrapping_add(1));

            if seq == last_seq {
                continue;
            }
            last_seq = seq;

            let Ok(txt) = clipboard_win::get_clipboard_string() else {
                continue;
            };
            if !txt.starts_with("magnet:?") {
                continue;
            }
            let Ok(parsed) = magnet::parse(&txt) else {
                continue;
            };
            if dismissed.contains(parsed.infohash.as_str()) {
                continue;
            }
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
            dismissed.insert(parsed.infohash.as_str().to_string());
        }
    });
}
