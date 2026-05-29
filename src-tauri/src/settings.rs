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
    pub category_map: SerCategoryMap,
}

fn default_theme() -> String { "system".into() }
fn default_max_active() -> u32 { 3 }

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
            let bytes = std::fs::read(&path)?;
            serde_json::from_slice(&bytes).unwrap_or_default()
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
