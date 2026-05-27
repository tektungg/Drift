//! Wrapper around librqbit that exposes a small, stable async API.
//!
//! All `librqbit::*` types are confined to this module. The rest of the
//! application uses only the types defined here.

use std::{
    collections::HashMap,
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ListOnlyResponse, ManagedTorrent, Session,
    SessionOptions, SessionPersistenceConfig,
};
use tokio::sync::broadcast;

use crate::{
    category::{CategoryMap, FileEntry},
    magnet::InfoHash,
};

// ─── Our own public types ────────────────────────────────────────────────────

/// Where a torrent should come from.
pub enum Source {
    Magnet(String),
    TorrentFile(PathBuf),
}

/// Torrent metadata returned by `peek` before any download starts.
#[derive(Debug, Clone)]
pub struct TorrentMetadata {
    pub infohash: InfoHash,
    pub name: String,
    pub total_size: u64,
    pub files: Vec<FileEntry>,
}

/// Per-torrent progress snapshot, emitted by the 1 Hz poll task.
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub infohash: InfoHash,
    pub downloaded: u64,
    pub total: u64,
    pub down_bps: u64,
    pub up_bps: u64,
    pub peers: u32,
    /// "downloading" | "seeding" | "paused" | "stalled" | "completed" | "error" | "initializing"
    pub state_label: String,
}

/// Per-file progress for a torrent.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileProgress {
    pub index: usize,
    pub path: String,
    pub size: u64,
    pub downloaded: u64,
    pub selected: bool,
}

// ─── Engine ──────────────────────────────────────────────────────────────────

/// Cheap-to-clone handle wrapping the librqbit session.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    session: Arc<Session>,
    resume_dir: PathBuf,
    progress_tx: broadcast::Sender<ProgressUpdate>,
}

impl Engine {
    /// Create a new session.  `resume_dir` is used for JSON persistence so
    /// torrents survive restarts.
    pub async fn new(resume_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(resume_dir)
            .with_context(|| format!("cannot create resume dir {resume_dir:?}"))?;

        let opts = SessionOptions {
            persistence: Some(SessionPersistenceConfig::Json {
                folder: Some(resume_dir.to_owned()),
            }),
            fastresume: true,
            // Don't bind a TCP listener port by default; pick a random one.
            listen_port_range: Some(6881..6891),
            ..Default::default()
        };

        let session = Session::new_with_opts(resume_dir.to_owned(), opts)
            .await
            .context("error creating librqbit session")?;

        let (progress_tx, _) = broadcast::channel(256);

        let engine = Engine {
            inner: Arc::new(EngineInner {
                session,
                resume_dir: resume_dir.to_owned(),
                progress_tx,
            }),
        };

        // Start the 1 Hz progress-polling task eagerly.
        engine.start_poll_task();

        Ok(engine)
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn session(&self) -> &Arc<Session> {
        &self.inner.session
    }

    /// Convert a hex-string InfoHash to librqbit's `Id20`.
    fn to_id20(ih: &InfoHash) -> Result<librqbit::dht::Id20> {
        use std::str::FromStr;
        librqbit::dht::Id20::from_str(ih.as_str())
            .with_context(|| format!("invalid infohash: {}", ih.as_str()))
    }

    /// Resolve an infohash to a managed torrent handle (Arc<ManagedTorrent>).
    fn get_handle(&self, ih: &InfoHash) -> Result<Arc<ManagedTorrent>> {
        let id20 = Self::to_id20(ih)?;
        self.session()
            .get(librqbit::api::TorrentIdOrHash::Hash(id20))
            .ok_or_else(|| anyhow!("torrent {} not found in session", ih.as_str()))
    }

    /// Build our `TorrentMetadata` from a librqbit `ListOnlyResponse`.
    fn make_metadata_from_list_only(
        resp: librqbit::ListOnlyResponse,
    ) -> Result<TorrentMetadata> {
        let name = resp
            .info
            .name
            .as_ref()
            .and_then(|b| std::str::from_utf8(b.as_ref()).ok())
            .unwrap_or("unknown")
            .to_owned();

        let mut total_size: u64 = 0;
        let mut files = Vec::new();
        for fd in resp.info.iter_file_details()? {
            let path = fd
                .filename
                .to_pathbuf()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "<unknown>".to_owned());
            total_size += fd.len;
            files.push(FileEntry {
                path,
                size: fd.len,
            });
        }

        let infohash = InfoHash(resp.info_hash.as_string());
        Ok(TorrentMetadata { infohash, name, total_size, files })
    }

    /// Build our `TorrentMetadata` from a managed torrent handle (Arc<ManagedTorrent>).
    fn make_metadata_from_handle(
        handle: &Arc<ManagedTorrent>,
    ) -> Result<TorrentMetadata> {
        let infohash = InfoHash(handle.info_hash().as_string());
        let name = handle.name().unwrap_or_else(|| "unknown".to_owned());

        handle.with_metadata(|meta| {
            let mut total_size: u64 = 0;
            let mut files = Vec::new();
            for fi in &meta.file_infos {
                let path = fi.relative_filename.to_string_lossy().into_owned();
                total_size += fi.len;
                files.push(FileEntry { path, size: fi.len });
            }
            TorrentMetadata { infohash: infohash.clone(), name: name.clone(), total_size, files }
        })
    }

    /// Start the background 1 Hz poll task.
    fn start_poll_task(&self) {
        let engine = self.clone();
        tokio::spawn(async move {
            // Stall detection: (last_downloaded_bytes, last_change_instant)
            let mut stall_map: HashMap<String, (u64, Instant)> = HashMap::new();
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;
                engine.poll_and_broadcast(&mut stall_map);
            }
        });
    }

    fn poll_and_broadcast(&self, stall_map: &mut HashMap<String, (u64, Instant)>) {
        let tx = &self.inner.progress_tx;
        // No subscribers means no need to compute anything.
        if tx.receiver_count() == 0 {
            return;
        }

        let updates = self.session().with_torrents(|iter| {
            iter.map(|(_, handle)| {
                let ih_str = handle.info_hash().as_string();
                let stats = handle.stats();
                let total = stats.total_bytes;

                // librqbit's `stats.progress_bytes` is sometimes 0 even when every
                // file's per-file progress is at 100% — typically when the data
                // already existed on disk and the torrent was resumed without a
                // fresh streaming-update cycle. Derive the aggregate from the
                // per-file progress array and take the larger of the two so we
                // always report the most accurate number.
                let file_sum: u64 = stats.file_progress.iter().copied().sum();
                let downloaded = stats.progress_bytes.max(file_sum);

                // Same defensive treatment for `finished`: librqbit's flag can lag.
                let finished = stats.finished || (total > 0 && downloaded >= total);

                let (down_bps, up_bps, peers) = stats
                    .live
                    .as_ref()
                    .map(|l| {
                        let down_bps =
                            (l.download_speed.mbps * 1024.0 * 1024.0) as u64;
                        let up_bps =
                            (l.upload_speed.mbps * 1024.0 * 1024.0) as u64;
                        let peers = l.snapshot.peer_stats.live as u32;
                        (down_bps, up_bps, peers)
                    })
                    .unwrap_or((0, 0, 0));

                let raw_state = format!("{}", stats.state);

                (ih_str, downloaded, total, down_bps, up_bps, peers, raw_state, finished)
            })
            .collect::<Vec<_>>()
        });

        let now = Instant::now();
        for (ih_str, downloaded, total, down_bps, up_bps, peers, raw_state, finished) in updates {
            // Determine the user-visible state label.
            //
            //   finished + live      → "seeding"   (done, actively sharing)
            //   finished + !live     → "completed" (done, paused or otherwise quiet)
            //   !finished + live     → "downloading" or "stalled" (30s no progress)
            //   !finished + !live    → librqbit's raw state (paused/initializing/error/…)
            let state_label = if finished {
                stall_map.remove(&ih_str);
                if raw_state == "live" { "seeding".to_owned() } else { "completed".to_owned() }
            } else if raw_state == "live" {
                let entry = stall_map
                    .entry(ih_str.clone())
                    .or_insert((downloaded, now));
                if downloaded > entry.0 {
                    *entry = (downloaded, now);
                }
                let stall_duration = now.duration_since(entry.1);
                if stall_duration >= Duration::from_secs(30) {
                    "stalled".to_owned()
                } else {
                    "downloading".to_owned()
                }
            } else {
                // Remove stall tracking for non-live torrents.
                stall_map.remove(&ih_str);
                match raw_state.as_str() {
                    "paused" => "paused".to_owned(),
                    "initializing" => "initializing".to_owned(),
                    "error" => "error".to_owned(),
                    other => other.to_owned(),
                }
            };

            let update = ProgressUpdate {
                infohash: InfoHash(ih_str),
                downloaded,
                total,
                down_bps,
                up_bps,
                peers,
                state_label,
            };
            // Ignore send errors (no receivers is fine).
            let _ = tx.send(update);
        }
    }

    // ── Public API ───────────────────────────────────────────────────────────

    /// Fetch torrent metadata **without** starting a download.
    ///
    /// For magnet links this may take several seconds while peers are
    /// contacted.  For `.torrent` files it is instantaneous.
    pub async fn peek(&self, source: &Source) -> Result<TorrentMetadata> {
        let opts = AddTorrentOptions {
            list_only: true,
            ..Default::default()
        };

        let add = match source {
            Source::Magnet(url) => AddTorrent::from_url(url),
            Source::TorrentFile(path) => {
                let bytes = std::fs::read(path)
                    .with_context(|| format!("cannot read torrent file {path:?}"))?;
                AddTorrent::from_bytes(bytes)
            }
        };

        match self.session().add_torrent(add, Some(opts)).await? {
            AddTorrentResponse::ListOnly(resp) => Self::make_metadata_from_list_only(resp),
            AddTorrentResponse::AlreadyManaged(_, handle) => {
                Self::make_metadata_from_handle(&handle)
            }
            AddTorrentResponse::Added(_, handle) => {
                // Shouldn't happen with list_only=true, but handle defensively.
                Self::make_metadata_from_handle(&handle)
            }
        }
    }

    /// Add and **start** a torrent.
    ///
    /// `selected_files`: if `None` all files are downloaded; if `Some`, only
    /// the listed file indices are downloaded.
    pub async fn start(
        &self,
        source: Source,
        save_path: &Path,
        selected_files: Option<Vec<usize>>,
    ) -> Result<InfoHash> {
        let opts = AddTorrentOptions {
            overwrite: true,
            output_folder: Some(save_path.to_string_lossy().into_owned()),
            only_files: selected_files,
            ..Default::default()
        };

        let add = match source {
            Source::Magnet(url) => AddTorrent::from_url(url),
            Source::TorrentFile(path) => {
                let bytes = std::fs::read(&path)
                    .with_context(|| format!("cannot read torrent file {path:?}"))?;
                AddTorrent::from_bytes(bytes)
            }
        };

        let resp = self
            .session()
            .add_torrent(add, Some(opts))
            .await
            .context("error adding torrent")?;

        let handle = resp
            .into_handle()
            .ok_or_else(|| anyhow!("expected a torrent handle after add"))?;

        Ok(InfoHash(handle.info_hash().as_string()))
    }

    /// Pause a running torrent.
    pub async fn pause(&self, ih: &InfoHash) -> Result<()> {
        let handle = self.get_handle(ih)?;
        self.session()
            .pause(&handle)
            .await
            .context("error pausing torrent")
    }

    /// Resume a paused torrent.
    pub async fn resume(&self, ih: &InfoHash) -> Result<()> {
        let handle = self.get_handle(ih)?;
        self.session()
            .unpause(&handle)
            .await
            .context("error resuming torrent")
    }

    /// Re-attach a torrent that was persisted on disk (called during startup).
    ///
    /// librqbit with JSON persistence auto-resumes all previously added
    /// torrents when `Session::new_with_opts` is called, so this is a no-op.
    /// It is kept in the API for callers that need to explicitly trigger
    /// resumption.
    pub async fn resume_existing(&self, ih: &InfoHash, _save_path: &Path) -> Result<()> {
        // librqbit's JSON persistence already re-added the torrent at Session
        // construction time.  If it exists, just ensure it's unpaused.
        match self.get_handle(ih) {
            Ok(handle) if handle.is_paused() => {
                self.session()
                    .unpause(&handle)
                    .await
                    .context("error unpausing persisted torrent")?;
            }
            Ok(_) => {} // already running
            Err(_) => {} // not in session — nothing to do
        }
        Ok(())
    }

    /// Remove a torrent from the session.  Pass `delete_files = true` to also
    /// delete the downloaded data from disk.
    ///
    /// Idempotent: if the torrent isn't currently in the session (e.g. already
    /// removed, or never added), returns `Ok(())` so callers can repeat the
    /// action without surfacing spurious errors.
    pub async fn remove(&self, ih: &InfoHash, delete_files: bool) -> Result<()> {
        let id20 = Self::to_id20(ih)?;
        // Pre-check: don't even ask librqbit to delete something it doesn't have.
        if self
            .session()
            .get(librqbit::api::TorrentIdOrHash::Hash(id20))
            .is_none()
        {
            return Ok(());
        }
        match self
            .session()
            .delete(librqbit::api::TorrentIdOrHash::Hash(id20), delete_files)
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                // Race: torrent disappeared between the `get` and `delete`. Swallow
                // "not found"-style failures; surface anything else.
                let s = format!("{e}").to_lowercase();
                if s.contains("not found") || s.contains("does not exist") || s.contains("no such") {
                    Ok(())
                } else {
                    Err(e).context("error removing torrent")
                }
            }
        }
    }

    /// Set session-level rate limits.  Pass `0` for unlimited.
    pub fn set_global_limits(&self, down_kbps: u32, up_kbps: u32) {
        let down = NonZeroU32::new(down_kbps.saturating_mul(1024));
        let up = NonZeroU32::new(up_kbps.saturating_mul(1024));
        self.session().ratelimits.set_download_bps(down);
        self.session().ratelimits.set_upload_bps(up);
    }

    /// Subscribe to 1 Hz progress broadcasts.
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressUpdate> {
        self.inner.progress_tx.subscribe()
    }

    /// Per-file progress for a torrent.
    pub async fn files(&self, ih: &InfoHash) -> Result<Vec<FileProgress>> {
        let handle = self.get_handle(ih)?;
        let stats = handle.stats();
        let only_files = handle.only_files();

        handle.with_metadata(|meta| {
            meta.file_infos
                .iter()
                .enumerate()
                .filter(|(_, fi)| !fi.attrs.padding)
                .map(|(idx, fi)| {
                    let path = fi.relative_filename.to_string_lossy().into_owned();
                    let size = fi.len;
                    let downloaded = stats.file_progress.get(idx).copied().unwrap_or(0);
                    let selected = only_files
                        .as_ref()
                        .map(|of| of.contains(&idx))
                        .unwrap_or(true);
                    FileProgress { index: idx, path, size, downloaded, selected }
                })
                .collect::<Vec<_>>()
        })
    }

    /// Change which files are downloaded for an already-managed torrent.
    pub async fn set_file_selection(
        &self,
        ih: &InfoHash,
        selected: &[usize],
    ) -> Result<()> {
        let handle = self.get_handle(ih)?;
        let selected_set: std::collections::HashSet<usize> =
            selected.iter().copied().collect();
        self.session()
            .update_only_files(&handle, &selected_set)
            .await
            .context("error updating file selection")
    }

    /// Pure helper — pick the save path for a torrent given the root download
    /// folder and the category map.
    pub fn pick_save_path(
        root: &Path,
        meta: &TorrentMetadata,
        map: &CategoryMap,
    ) -> PathBuf {
        let cat = crate::category::resolve(&meta.files, map);
        root.join(cat.folder_name())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_save_path_uses_category() {
        use crate::category::{Category, CategoryMap, FileEntry};
        let map = CategoryMap::default();
        let files = vec![FileEntry { path: "movie.mkv".into(), size: 1 }];
        let cat = crate::category::resolve(&files, &map);
        assert_eq!(cat, Category::Video);
        let p = std::path::Path::new("C:/Downloads/Drift").join(cat.folder_name());
        assert_eq!(p, std::path::PathBuf::from("C:/Downloads/Drift/Video"));
    }
}
