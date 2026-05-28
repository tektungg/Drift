use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct TorrentDto {
    pub infohash: String,
    pub name: String,
    pub downloaded: u64,
    pub total: u64,
    pub uploaded: u64,
    pub down_bps: u64,
    pub up_bps: u64,
    pub peers: u32,
    pub added_at: i64,
    pub state_label: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddRequest {
    pub source: String,           // either a magnet URI or a path to a .torrent file
    pub override_path: Option<String>,
    pub selected_files: Option<Vec<usize>>,
}

#[derive(Serialize, Clone)]
pub struct ToastEvent {
    pub kind: String,             // "info" | "error"
    pub message: String,
}
