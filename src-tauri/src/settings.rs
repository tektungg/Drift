use crate::category::CategoryMap;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub download_root: PathBuf,
    pub download_kbps: u32,
    pub upload_kbps: u32,
    pub clipboard_watch: bool,
    pub start_with_windows: bool,
    pub close_to_tray: bool,
    /// UI theme: "system" | "light" | "dark". Frontend-only; defaulted so
    /// config.json files written before this field existed still load.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Register Drift as the Windows handler for magnet: links. Opt-in
    /// (default false) so installing Drift doesn't hijack the user's
    /// existing torrent client. Defaulted for backward-compatible configs.
    #[serde(default)]
    pub magnet_handler: bool,
    /// Max torrents downloading at once; the rest wait in the queue.
    /// 0 = unlimited. Defaulted for backward-compatible configs.
    #[serde(default = "default_max_active")]
    pub max_active_downloads: u32,
    /// Stop seeding when uploaded / total_size reaches this. 0 = unlimited.
    #[serde(default)]
    pub seed_ratio_limit: f64,
    /// Stop seeding this many minutes after a torrent finishes. 0 = unlimited.
    #[serde(default)]
    pub seed_time_limit_mins: u32,
    /// Show a Windows notification when a download completes.
    #[serde(default = "default_true")]
    pub notify_on_complete: bool,
    pub category_map: SerCategoryMap,
}

fn default_theme() -> String { "system".into() }
fn default_max_active() -> u32 { 3 }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerCategoryMap {
    pub video: Vec<String>,
    pub audio: Vec<String>,
    pub documents: Vec<String>,
    pub compressed: Vec<String>,
    pub programs: Vec<String>,
    pub images: Vec<String>,
}

impl From<CategoryMap> for SerCategoryMap {
    fn from(c: CategoryMap) -> Self {
        Self { video: c.video, audio: c.audio, documents: c.documents,
               compressed: c.compressed, programs: c.programs, images: c.images }
    }
}
impl From<SerCategoryMap> for CategoryMap {
    fn from(c: SerCategoryMap) -> Self {
        Self { video: c.video, audio: c.audio, documents: c.documents,
               compressed: c.compressed, programs: c.programs, images: c.images }
    }
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:/"));
        Self {
            download_root: home.join("Downloads").join("Drift"),
            download_kbps: 0,
            upload_kbps: 0,
            clipboard_watch: true,
            start_with_windows: false,
            close_to_tray: true,
            theme: "system".into(),
            magnet_handler: false,
            max_active_downloads: 3,
            seed_ratio_limit: 0.0,
            seed_time_limit_mins: 0,
            notify_on_complete: true,
            category_map: CategoryMap::default().into(),
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    inner: RwLock<Config>,
}

impl SettingsStore {
    pub fn load_or_init(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join("config.json");
        let inner = if path.exists() {
            crate::persist::load_json_or_recover::<Config>(&path)
        } else {
            let c = Config::default();
            std::fs::write(&path, serde_json::to_vec_pretty(&c)?)?;
            c
        };
        Ok(Self { path, inner: RwLock::new(inner) })
    }

    pub fn get(&self) -> Config { self.inner.read().unwrap().clone() }

    pub fn replace(&self, new: Config) -> Result<()> {
        {
            let mut w = self.inner.write().unwrap();
            *w = new.clone();
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&new)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn max_active_defaults_to_three() {
        let c = Config::default();
        assert_eq!(c.max_active_downloads, 3);
    }

    #[test]
    fn legacy_config_loads_with_new_defaults() {
        // A config.json written before 0.6.0 — none of the new fields present.
        let json = r#"{
            "download_root": "C:/D",
            "download_kbps": 0,
            "upload_kbps": 0,
            "clipboard_watch": true,
            "start_with_windows": false,
            "close_to_tray": true,
            "category_map": {"video":[],"audio":[],"documents":[],"compressed":[],"programs":[],"images":[]}
        }"#;
        let c: Config = serde_json::from_str(json).unwrap();
        assert_eq!(c.seed_ratio_limit, 0.0);
        assert_eq!(c.seed_time_limit_mins, 0);
        assert!(c.notify_on_complete);
    }

    #[test]
    fn defaults_then_persist() {
        let d = tempdir().unwrap();
        let s = SettingsStore::load_or_init(d.path()).unwrap();
        let mut c = s.get();
        c.download_kbps = 1024;
        s.replace(c).unwrap();
        let s2 = SettingsStore::load_or_init(d.path()).unwrap();
        assert_eq!(s2.get().download_kbps, 1024);
    }
}
