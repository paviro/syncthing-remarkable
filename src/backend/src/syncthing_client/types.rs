use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Public Payload Types (moved from root types.rs)
// ============================================================================

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

// ============================================================================
// Internal Syncthing Client Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SyncthingConfig {
    #[serde(default)]
    pub folders: Vec<FolderConfig>,
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FolderConfig {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub paused: Option<bool>,
    #[serde(default)]
    pub devices: Vec<FolderDevice>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FolderDevice {
    #[serde(rename = "deviceID")]
    pub device_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceConfig {
    #[serde(rename = "deviceID")]
    pub device_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub paused: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ConnectionsResponse {
    #[serde(default)]
    pub connections: HashMap<String, ConnectionState>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ConnectionState {
    #[serde(default)]
    pub connected: bool,
    #[serde(default)]
    pub paused: bool,
    #[serde(default, rename = "clientVersion")]
    pub client_version: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default, rename = "lastSeen")]
    pub last_seen: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncthingEvent {
    pub id: u64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub time: String,
    pub data: Value,
}

#[derive(Debug, Deserialize)]
pub struct RemoteCompletion {
    #[allow(dead_code)]
    pub completion: Option<f64>,
    #[serde(rename = "needBytes")]
    pub need_bytes: Option<u64>,
}

#[derive(Default, Clone)]
pub struct PeerProgress {
    pub total_completion: f64,
    pub completion_samples: u32,
    pub total_need_bytes: u64,
    pub folders: Vec<PeerFolderState>,
}

pub struct FolderStateInfo {
    pub label: String,
    pub code: FolderStateCode,
}

impl PeerProgress {
    pub fn record(&mut self, folder: &FolderConfig, completion: &RemoteCompletion) {
        if let Some(value) = completion.completion {
            self.total_completion += value;
            self.completion_samples = self.completion_samples.saturating_add(1);
        }
        if let Some(need) = completion.need_bytes {
            self.total_need_bytes = self.total_need_bytes.saturating_add(need);
        }
        self.folders.push(PeerFolderState {
            folder_id: folder.id.clone(),
            folder_label: folder.label.clone().unwrap_or_else(|| folder.id.clone()),
            completion: completion.completion,
            need_bytes: completion.need_bytes,
        });
    }

    pub fn avg_completion(&self) -> Option<f64> {
        if self.completion_samples == 0 {
            None
        } else {
            let mut average = self.total_completion / self.completion_samples as f64;
            if self.total_need_bytes > 0 && average > 99.99 {
                average = 99.99;
            }
            if average > 100.0 {
                average = 100.0;
            }
            Some(average)
        }
    }

    pub fn outstanding_need(&self) -> Option<u64> {
        if self.total_need_bytes > 0 {
            Some(self.total_need_bytes)
        } else {
            None
        }
    }
}

impl FolderStateInfo {
    pub fn new(label: impl Into<String>, code: FolderStateCode) -> Self {
        Self {
            label: label.into(),
            code,
        }
    }
}

impl SyncthingEvent {
    pub fn folder_id(&self) -> Option<&str> {
        self.data.get("folder").and_then(|v| v.as_str())
    }

    pub fn file_name(&self) -> Option<String> {
        if let Some(item) = self.data.get("item").and_then(|v| v.as_str()) {
            return Some(item.to_string());
        }
        if let Some(file) = self.data.get("file").and_then(|v| v.as_str()) {
            return Some(file.to_string());
        }
        if let Some(items) = self.data.get("items").and_then(|v| v.as_array()) {
            for entry in items {
                if let Some(path) = entry
                    .get("path")
                    .or_else(|| entry.get("item"))
                    .or_else(|| entry.get("file"))
                    .and_then(|v| v.as_str())
                {
                    return Some(path.to_string());
                }
            }
        }
        if let Some(files) = self.data.get("files").and_then(|v| v.as_array()) {
            for entry in files {
                if let Some(path) = entry
                    .get("path")
                    .or_else(|| entry.get("item"))
                    .or_else(|| entry.get("file"))
                    .and_then(|v| v.as_str())
                {
                    return Some(path.to_string());
                }
            }
        }
        None
    }

    pub fn action(&self) -> Option<String> {
        if let Some(action) = self.data.get("action").and_then(|v| v.as_str()) {
            return Some(action.to_string());
        }
        if let Some(items) = self.data.get("items").and_then(|v| v.as_array()) {
            for entry in items {
                if let Some(action) = entry.get("action").and_then(|v| v.as_str()) {
                    return Some(action.to_string());
                }
            }
        }
        None
    }

    pub fn origin(&self) -> Option<String> {
        self.data
            .get("device")
            .or_else(|| self.data.get("peerID"))
            .or_else(|| self.data.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

