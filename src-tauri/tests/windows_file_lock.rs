//! Verifies the Windows file-sharing behavior the vendored-librqbit storage
//! patch relies on (see `vendor/librqbit/src/storage/filesystem/`):
//!
//! A process holding a **read-only** handle does NOT prevent another process
//! from opening the file for execution, but a **read+write** handle DOES. The
//! patch makes the filesystem storage hold only a read-only handle once a file
//! is written (taking a transient write handle only during writes), so a
//! completed `.exe`/`.bat` can be launched while the torrent keeps seeding.
//!
//! This test is deterministic and Windows-only.
#![cfg(windows)]

use std::fs::OpenOptions;
use std::os::windows::fs::OpenOptionsExt;

const FILE_SHARE_READ: u32 = 0x1;
const FILE_SHARE_DELETE: u32 = 0x4;

/// Open the file the way the Windows image loader opens an `.exe`: no
/// FILE_SHARE_WRITE, which makes the open fail if any existing handle holds
/// write access to the file.
fn exec_style_open(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .open(path)
}

#[test]
fn readonly_handle_allows_execution_but_write_handle_blocks_it() {
    let dir = std::env::temp_dir().join(format!("drift_lock_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("payload.exe");
    std::fs::write(&path, b"MZ\x90\x00").unwrap();

    // Patched-storage behavior: only a READ-ONLY handle is held -> runnable.
    {
        let _ro = OpenOptions::new().read(true).open(&path).unwrap();
        assert!(
            exec_style_open(&path).is_ok(),
            "a read-only handle must NOT block execution (this is the fix)"
        );
    }

    // Upstream behavior (the bug): a READ+WRITE handle is held -> blocked.
    {
        let _rw = OpenOptions::new().read(true).write(true).open(&path).unwrap();
        assert!(
            exec_style_open(&path).is_err(),
            "a read+write handle must block execution (this was the bug)"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
