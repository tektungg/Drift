use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use anyhow::Context;
use parking_lot::RwLock;

/// A handle to a torrent content file.
///
/// PATCHED (Drift fork of librqbit 8.1.1): `file` holds a **read-only** handle
/// used for reads (hash checks + seeding), and `path` is the absolute path used
/// to open a **short-lived** read+write handle only while writing.
///
/// Upstream librqbit kept a single persistent read+write handle for the entire
/// torrent lifetime. On Windows, any process holding a *write* handle on a file
/// prevents other processes from executing/opening it — so a completed `.exe`
/// couldn't be launched while the torrent kept seeding. By holding only a
/// read-only handle (and taking a write handle transiently during writes, which
/// only happen while a file is still incomplete), completed files become
/// runnable while seeding continues.
#[derive(Debug)]
pub(crate) struct OpenedFile {
    pub file: RwLock<Option<File>>,
    pub path: Option<PathBuf>,
}

impl OpenedFile {
    pub fn new(f: File, path: PathBuf) -> Self {
        Self {
            file: RwLock::new(Some(f)),
            path: Some(path),
        }
    }

    pub fn new_dummy() -> Self {
        Self {
            file: RwLock::new(None),
            path: None,
        }
    }

    pub fn take(&self) -> anyhow::Result<Option<File>> {
        let mut f = self.file.write();
        Ok(f.take())
    }

    pub fn take_clone(&self) -> anyhow::Result<Self> {
        let f = self.take()?;
        Ok(Self {
            file: RwLock::new(f),
            path: self.path.clone(),
        })
    }

    /// Open a short-lived read+write handle for this file. Used for writes and
    /// length changes. Errors for padding/dummy files (which have no path).
    pub fn open_writable(&self) -> anyhow::Result<File> {
        let path = self
            .path
            .as_ref()
            .context("cannot open a padding/dummy file for writing")?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("error opening {path:?} for writing"))
    }
}
