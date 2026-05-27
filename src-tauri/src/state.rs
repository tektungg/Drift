use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TorrentState { Downloading, Seeding, Paused, Completed, Stalled }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentRecord {
    pub infohash: String,
    pub display_name: String,
    pub save_path: PathBuf,
    pub state: TorrentState,
    pub added_at: i64,
    pub total_size: u64,
    pub selected_files: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedState {
    pub torrents: Vec<TorrentRecord>,
}

pub struct StateStore {
    path: PathBuf,
    inner: Mutex<PersistedState>,
}

impl StateStore {
    pub fn load_or_init(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir).with_context(|| format!("create {dir:?}"))?;
        let path = dir.join("state.json");
        let inner = if path.exists() {
            let bytes = std::fs::read(&path)?;
            serde_json::from_slice(&bytes).unwrap_or_default()
        } else { PersistedState::default() };
        Ok(Self { path, inner: Mutex::new(inner) })
    }

    pub fn snapshot(&self) -> PersistedState { self.inner.lock().unwrap().clone() }

    pub fn upsert(&self, rec: TorrentRecord) -> Result<()> {
        let mut s = self.inner.lock().unwrap();
        if let Some(existing) = s.torrents.iter_mut().find(|t| t.infohash == rec.infohash) {
            *existing = rec;
        } else { s.torrents.push(rec); }
        self.flush(&s)
    }

    pub fn remove(&self, infohash: &str) -> Result<()> {
        let mut s = self.inner.lock().unwrap();
        s.torrents.retain(|t| t.infohash != infohash);
        self.flush(&s)
    }

    pub fn contains(&self, infohash: &str) -> bool {
        self.inner.lock().unwrap().torrents.iter().any(|t| t.infohash == infohash)
    }

    fn flush(&self, s: &PersistedState) -> Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn rec(ih: &str) -> TorrentRecord {
        TorrentRecord {
            infohash: ih.into(),
            display_name: "x".into(),
            save_path: PathBuf::from("C:/"),
            state: TorrentState::Downloading,
            added_at: 0, total_size: 0, selected_files: None,
        }
    }

    #[test]
    fn round_trip() {
        let d = tempdir().unwrap();
        let s = StateStore::load_or_init(d.path()).unwrap();
        s.upsert(rec("aaa")).unwrap();
        s.upsert(rec("bbb")).unwrap();
        let s2 = StateStore::load_or_init(d.path()).unwrap();
        assert_eq!(s2.snapshot().torrents.len(), 2);
    }
    #[test]
    fn upsert_replaces() {
        let d = tempdir().unwrap();
        let s = StateStore::load_or_init(d.path()).unwrap();
        s.upsert(rec("aaa")).unwrap();
        let mut r2 = rec("aaa"); r2.display_name = "updated".into();
        s.upsert(r2).unwrap();
        assert_eq!(s.snapshot().torrents.len(), 1);
        assert_eq!(s.snapshot().torrents[0].display_name, "updated");
    }
    #[test]
    fn remove_works() {
        let d = tempdir().unwrap();
        let s = StateStore::load_or_init(d.path()).unwrap();
        s.upsert(rec("aaa")).unwrap();
        s.remove("aaa").unwrap();
        assert!(s.snapshot().torrents.is_empty());
    }
    #[test]
    fn contains_check() {
        let d = tempdir().unwrap();
        let s = StateStore::load_or_init(d.path()).unwrap();
        s.upsert(rec("aaa")).unwrap();
        assert!(s.contains("aaa"));
        assert!(!s.contains("bbb"));
    }
}
