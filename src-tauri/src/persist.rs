//! Crash-safe JSON loading. A corrupt store file is backed up and the caller
//! gets a default, instead of silently wiping data the next flush would
//! overwrite.

use std::path::Path;

/// Load and parse JSON at `path`.
/// - missing file → `T::default()`
/// - valid file → parsed value
/// - exists but unparseable → rename to `<name>.corrupt-<epoch_ms>.bak`,
///   warn, return `T::default()`
/// - exists but unreadable → warn, return `T::default()` (no rename — we won't
///   destroy a file we couldn't even read)
pub fn load_json_or_recover<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    if !path.exists() {
        return T::default();
    }
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<T>(&bytes) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("corrupt JSON at {path:?}: {e}; backing up and resetting");
                backup_corrupt(path);
                T::default()
            }
        },
        Err(e) => {
            tracing::warn!("cannot read {path:?}: {e}; using defaults");
            T::default()
        }
    }
}

fn backup_corrupt(path: &Path) {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let name = path.file_name().and_then(|f| f.to_str()).unwrap_or("state.json");
    let bak = path.with_file_name(format!("{name}.corrupt-{ms}.bak"));
    match std::fs::rename(path, &bak) {
        Ok(()) => tracing::info!("backed up corrupt file to {bak:?}"),
        Err(e) => tracing::warn!("could not back up corrupt file {path:?}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    struct Doc { n: u32 }

    #[test]
    fn missing_file_returns_default() {
        let d = tempdir().unwrap();
        let got: Doc = load_json_or_recover(&d.path().join("none.json"));
        assert_eq!(got, Doc::default());
    }

    #[test]
    fn valid_file_parses() {
        let d = tempdir().unwrap();
        let p = d.path().join("doc.json");
        std::fs::write(&p, br#"{"n":7}"#).unwrap();
        let got: Doc = load_json_or_recover(&p);
        assert_eq!(got, Doc { n: 7 });
    }

    #[test]
    fn corrupt_file_is_backed_up_and_reset() {
        let d = tempdir().unwrap();
        let p = d.path().join("doc.json");
        std::fs::write(&p, b"{ this is not json").unwrap();
        let got: Doc = load_json_or_recover(&p);
        assert_eq!(got, Doc::default());
        // original is gone (renamed)…
        assert!(!p.exists());
        // …and exactly one .corrupt-*.bak remains
        let baks: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".corrupt-"))
            .collect();
        assert_eq!(baks.len(), 1);
    }
}
