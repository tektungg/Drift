use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
};

use anyhow::Context;
use tracing::warn;

use crate::{
    storage::StorageFactoryExt,
    torrent_state::{ManagedTorrentShared, TorrentMetadata},
};

use crate::storage::{StorageFactory, TorrentStorage};

use super::opened_file::OpenedFile;

#[derive(Default, Clone, Copy)]
pub struct FilesystemStorageFactory {}

impl StorageFactory for FilesystemStorageFactory {
    type Storage = FilesystemStorage;

    fn create(
        &self,
        shared: &ManagedTorrentShared,
        _metadata: &TorrentMetadata,
    ) -> anyhow::Result<FilesystemStorage> {
        Ok(FilesystemStorage {
            output_folder: shared.options.output_folder.clone(),
            opened_files: Default::default(),
        })
    }

    fn clone_box(&self) -> crate::storage::BoxStorageFactory {
        self.boxed()
    }
}

pub struct FilesystemStorage {
    pub(super) output_folder: PathBuf,
    pub(super) opened_files: Vec<OpenedFile>,
}

impl FilesystemStorage {
    pub(super) fn take_fs(&self) -> anyhow::Result<Self> {
        Ok(Self {
            opened_files: self
                .opened_files
                .iter()
                .map(|f| f.take_clone())
                .collect::<anyhow::Result<Vec<_>>>()?,
            output_folder: self.output_folder.clone(),
        })
    }
}

impl TorrentStorage for FilesystemStorage {
    fn pread_exact(&self, file_id: usize, offset: u64, buf: &mut [u8]) -> anyhow::Result<()> {
        let of = self.opened_files.get(file_id).context("no such file")?;
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::FileExt;
            Ok(of
                .file
                .read()
                .as_ref()
                .context("file is None")?
                .read_exact_at(buf, offset)?)
        }
        #[cfg(target_family = "windows")]
        {
            use std::os::windows::fs::FileExt;
            let g = of.file.read();
            let f = g.as_ref().context("file is None")?;
            f.seek_read(buf, offset)?;
            Ok(())
        }
        #[cfg(not(any(target_family = "unix", target_family = "windows")))]
        {
            use std::io::{Read, Seek, SeekFrom};
            let mut g = of.file.write();
            let mut f = g.as_ref().context("file is None")?;
            f.seek(SeekFrom::Start(offset))?;
            Ok(f.read_exact(buf)?)
        }
    }

    fn pwrite_all(&self, file_id: usize, offset: u64, buf: &[u8]) -> anyhow::Result<()> {
        // PATCHED (Drift): write through a short-lived read+write handle instead
        // of a persistent one, so finished files hold no write handle.
        let of = self.opened_files.get(file_id).context("no such file")?;
        let f = of.open_writable()?;
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::FileExt;
            Ok(f.write_all_at(buf, offset)?)
        }
        #[cfg(target_family = "windows")]
        {
            use std::os::windows::fs::FileExt;
            let mut written = 0usize;
            while written < buf.len() {
                let n = f.seek_write(&buf[written..], offset + written as u64)?;
                if n == 0 {
                    anyhow::bail!("seek_write wrote 0 bytes for file_id {file_id}");
                }
                written += n;
            }
            Ok(())
        }
        #[cfg(not(any(target_family = "unix", target_family = "windows")))]
        {
            use std::io::{Seek, SeekFrom, Write};
            let mut f = f;
            f.seek(SeekFrom::Start(offset))?;
            Ok(f.write_all(buf)?)
        }
    }

    fn remove_file(&self, _file_id: usize, filename: &Path) -> anyhow::Result<()> {
        Ok(std::fs::remove_file(self.output_folder.join(filename))?)
    }

    fn ensure_file_length(&self, file_id: usize, len: u64) -> anyhow::Result<()> {
        // PATCHED (Drift): set length via a short-lived read+write handle so no
        // persistent write handle is retained.
        let of = self.opened_files.get(file_id).context("no such file")?;
        Ok(of.open_writable()?.set_len(len)?)
    }

    fn take(&self) -> anyhow::Result<Box<dyn TorrentStorage>> {
        Ok(Box::new(Self {
            opened_files: self
                .opened_files
                .iter()
                .map(|f| f.take_clone())
                .collect::<anyhow::Result<Vec<_>>>()?,
            output_folder: self.output_folder.clone(),
        }))
    }

    fn remove_directory_if_empty(&self, path: &Path) -> anyhow::Result<()> {
        let path = self.output_folder.join(path);
        if !path.is_dir() {
            anyhow::bail!("cannot remove dir: {path:?} is not a directory")
        }
        if std::fs::read_dir(&path)?.count() == 0 {
            std::fs::remove_dir(&path).with_context(|| format!("error removing {path:?}"))
        } else {
            warn!("did not remove {path:?} as it was not empty");
            Ok(())
        }
    }

    fn init(
        &mut self,
        shared: &ManagedTorrentShared,
        metadata: &TorrentMetadata,
    ) -> anyhow::Result<()> {
        let mut files = Vec::<OpenedFile>::new();
        for file_details in metadata.file_infos.iter() {
            let mut full_path = self.output_folder.clone();
            let relative_path = &file_details.relative_filename;
            full_path.push(relative_path);

            if file_details.attrs.padding {
                files.push(OpenedFile::new_dummy());
                continue;
            };
            std::fs::create_dir_all(full_path.parent().context("bug: no parent")?)?;
            // PATCHED (Drift): ensure the file exists using a transient write
            // handle that we immediately drop, then keep only a READ-ONLY handle.
            // Writes reopen a short-lived read+write handle (see pwrite_all /
            // ensure_file_length). This avoids a persistent write handle, so a
            // completed file can be executed/opened by other apps while seeding.
            if shared.options.allow_overwrite {
                OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .read(true)
                    .write(true)
                    .open(&full_path)
                    .with_context(|| format!("error opening {full_path:?} in read/write mode"))?;
            } else {
                OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&full_path)
                    .with_context(|| {
                        format!(
                            "error creating a new file (because allow_overwrite = false) {:?}",
                            &full_path
                        )
                    })?;
            }
            let f = OpenOptions::new()
                .read(true)
                .open(&full_path)
                .with_context(|| format!("error opening {full_path:?} read-only"))?;
            files.push(OpenedFile::new(f, full_path));
        }

        self.opened_files = files;
        Ok(())
    }
}

// PATCHED (Drift): regression test proving completed files are NOT write-locked.
#[cfg(all(test, target_family = "windows"))]
mod drift_patch_tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    // dwShareMode flags. Omitting FILE_SHARE_WRITE means "while I hold this open,
    // no one may have it open with write access" — the open fails if any existing
    // handle holds WRITE. This mimics how the Windows image loader opens an .exe.
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_DELETE: u32 = 0x4;

    #[test]
    fn completed_file_is_not_write_locked_while_storage_open() {
        let dir = std::env::temp_dir().join(format!("drift_fs_patch_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("payload.bin");
        std::fs::write(&path, b"").unwrap();

        // Build the storage exactly as init() leaves it: a read-only handle + path.
        let ro = OpenOptions::new().read(true).open(&path).unwrap();
        let storage = FilesystemStorage {
            output_folder: dir.clone(),
            opened_files: vec![OpenedFile::new(ro, path.clone())],
        };

        // Simulate a finished download: allocate length, write the payload.
        storage.ensure_file_length(0, 5).unwrap();
        storage.pwrite_all(0, 0, b"hello").unwrap();

        // While the storage is STILL ALIVE (i.e. the torrent keeps seeding), an
        // execute-style open with no write-sharing must succeed. Against upstream
        // librqbit (which keeps a persistent read+write handle) this would fail
        // with a sharing violation.
        let exec_open = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
            .open(&path);
        assert!(
            exec_open.is_ok(),
            "completed file is still write-locked while seeding: {:?}",
            exec_open.err()
        );

        // Data round-trips through the read-only handle.
        let mut buf = [0u8; 5];
        storage.pread_exact(0, 0, &mut buf).unwrap();
        assert_eq!(&buf, b"hello");

        drop(storage);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
