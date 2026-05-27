use drift::engine::{Engine, Source};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
#[ignore = "requires src-tauri/tests/fixtures/test.torrent and network"]
async fn end_to_end_add_download_resume() {
    let tmp = tempfile::tempdir().unwrap();
    let resume_dir = tmp.path().join("resume");
    let dl_dir = tmp.path().join("dl");
    std::fs::create_dir_all(&dl_dir).unwrap();

    let engine = Engine::new(&resume_dir).await.unwrap();
    let torrent_path: PathBuf = "tests/fixtures/test.torrent".into();
    let src = Source::TorrentFile(torrent_path);

    let meta = engine.peek(&src).await.unwrap();
    assert!(!meta.files.is_empty());

    let ih = engine.start(src, &dl_dir, None).await.unwrap();

    // wait up to 5 minutes for completion
    let mut rx = engine.subscribe();
    let done = timeout(Duration::from_secs(300), async {
        loop {
            let u = rx.recv().await.unwrap();
            if u.infohash == ih && u.downloaded >= u.total { return; }
        }
    }).await;
    assert!(done.is_ok(), "torrent did not complete within 5 minutes");

    // pause + drop engine + reopen
    engine.pause(&ih).await.unwrap();
    drop(engine);
    let engine2 = Engine::new(&resume_dir).await.unwrap();
    let snap = engine2.subscribe();
    let first = timeout(Duration::from_secs(10), async {
        let mut rx = snap;
        rx.recv().await.unwrap()
    }).await.unwrap();
    assert_eq!(first.infohash, ih);
}
