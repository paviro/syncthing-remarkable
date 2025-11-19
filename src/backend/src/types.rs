use serde::Serialize;
use thiserror::Error;

// Re-export types from other modules for convenience
pub use crate::systemd::SystemdStatus;
pub use crate::syncthing_client::{FolderPayload, PeerPayload, SyncthingOverview};

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

