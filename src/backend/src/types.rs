use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("systemd error: {0}")]
    Systemd(String),
    #[error("syncthing api error: {0}")]
    Syncthing(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Syncthing API key not found")]
    MissingApiKey,
    #[error("config error: {0}")]
    Config(String),
}

#[derive(Debug, Serialize)]
pub struct StatusPayload {
    pub fetched_at: String,
    pub reason: String,
    pub systemd: SystemdStatus,
    pub syncthing: SyncthingOverview,
    pub folders: Vec<FolderPayload>,
    pub peers: Vec<PeerPayload>,
    pub gui_address: Option<String>,
}

#[derive(Debug, Serialize, Default, Clone, PartialEq, Eq)]
pub struct SystemdStatus {
    pub name: String,
    pub active_state: Option<String>,
    pub sub_state: Option<String>,
    pub unit_file_state: Option<String>,
    pub result: Option<String>,
    pub pid: Option<u32>,
    pub active_enter_timestamp: Option<String>,
    pub inactive_enter_timestamp: Option<String>,
    pub description: Option<String>,
    pub raw_excerpt: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct SyncthingOverview {
    pub available: bool,
    pub my_id: Option<String>,
    pub version: Option<String>,
    pub state: Option<String>,
    pub health: Option<String>,
    pub started_at: Option<String>,
    pub uptime_seconds: Option<f64>,
    pub sequence: Option<u64>,
    pub goroutine_count: Option<u64>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FolderStateCode {
    Unknown,
    Paused,
    Error,
    WaitingToScan,
    WaitingToSync,
    Scanning,
    PreparingToSync,
    Syncing,
    PendingChanges,
    UpToDate,
}

impl Default for FolderStateCode {
    fn default() -> Self {
        FolderStateCode::Unknown
    }
}

#[derive(Debug, Serialize)]
pub struct FolderPayload {
    pub id: String,
    pub label: String,
    pub path: Option<String>,
    pub state: String,
    pub state_code: FolderStateCode,
    pub state_raw: Option<String>,
    pub paused: bool,
    pub global_bytes: Option<u64>,
    pub in_sync_bytes: Option<u64>,
    pub need_bytes: Option<u64>,
    pub completion: f64,
    pub last_changes: Vec<FolderChange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peers_need_summary: Option<FolderPeerNeedSummary>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct FolderChange {
    pub name: String,
    pub action: String,
    pub when: String,
    pub origin: Option<String>,
}

#[derive(Debug, Serialize, Clone, Copy, Default)]
pub struct FolderPeerNeedSummary {
    pub peer_count: u32,
    pub need_bytes: u64,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct PeerFolderState {
    pub folder_id: String,
    pub folder_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_bytes: Option<u64>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct PeerPayload {
    pub id: String,
    pub name: String,
    pub connected: bool,
    pub paused: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<PeerFolderState>,
}
