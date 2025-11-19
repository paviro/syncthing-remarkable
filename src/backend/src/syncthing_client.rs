use std::collections::{HashMap, HashSet};
use std::env;
use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;

use crate::config::Config;
use crate::types::{FolderChange, FolderPayload, FolderStateCode, MonitorError, SyncthingOverview};

const RECENT_EVENTS_LIMIT: u32 = 200;
const RECENT_FILES_PER_FOLDER: usize = 4;

#[derive(Clone)]
pub struct SyncthingClient {
    api_key: String,
    http: Client,
    base_urls: Vec<String>,
    current_idx: usize,
}

pub struct SyncthingData {
    pub overview: SyncthingOverview,
    pub folders: Vec<FolderPayload>,
}

pub struct EventWaitResult {
    pub last_event_id: u64,
    pub has_updates: bool,
}

impl SyncthingClient {
    pub async fn discover(config: &Config) -> Result<Self, MonitorError> {
        let api_key = load_api_key(config).await?;
        let mut base_urls = Vec::new();
        if let Ok(custom) = env::var("SYNCTHING_API_URL") {
            let trimmed = custom.trim();
            if !trimmed.is_empty() {
                push_unique_url(&mut base_urls, trimmed.to_string());
            }
        }
        push_unique_url(&mut base_urls, "https://127.0.0.1:8384".to_string());
        push_unique_url(&mut base_urls, "http://127.0.0.1:8384".to_string());
        if base_urls.is_empty() {
            base_urls.push("http://127.0.0.1:8384".to_string());
        }

        let http = Client::builder()
            .timeout(Duration::from_secs(8))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(MonitorError::Http)?;

        Ok(Self {
            api_key,
            http,
            base_urls,
            current_idx: 0,
        })
    }

    pub async fn compose_payload(&mut self) -> Result<SyncthingData, MonitorError> {
        let status_value: Value = self.get_json("/rest/system/status").await?;
        let config: SyncthingConfig = self.get_json("/rest/config").await?;
        let folder_ids: HashSet<String> = config.folders.iter().map(|f| f.id.clone()).collect();
        let recent = self
            .recent_folder_changes(&folder_ids, RECENT_FILES_PER_FOLDER)
            .await?;
        let mut folders = Vec::new();

        for folder in &config.folders {
            let query = FolderStatusQuery {
                folder: folder.id.as_str(),
            };
            let status: Value = self.get_json_with_query("/rest/db/status", &query).await?;
            let last_changes = recent.get(&folder.id).cloned().unwrap_or_default();
            folders.push(FolderPayload::from_parts(folder, &status, last_changes));
        }

        Ok(SyncthingData {
            overview: SyncthingOverview::from_value(&status_value),
            folders,
        })
    }

    pub async fn wait_for_updates(
        &mut self,
        since: u64,
        timeout: Duration,
    ) -> Result<EventWaitResult, MonitorError> {
        let timeout_secs = timeout.as_secs().clamp(1, 300);
        let query = EventStreamQuery {
            since,
            limit: 1,
            timeout: timeout_secs,
            events: None,
        };
        let events: Vec<SyncthingEvent> = self.get_json_with_query("/rest/events", &query).await?;

        let mut last_event_id = since;
        for event in &events {
            if event.id > last_event_id {
                last_event_id = event.id;
            }
        }

        Ok(EventWaitResult {
            last_event_id,
            has_updates: !events.is_empty(),
        })
    }

    async fn recent_folder_changes(
        &mut self,
        allowed: &HashSet<String>,
        _per_folder: usize,
    ) -> Result<HashMap<String, Vec<FolderChange>>, MonitorError> {
        if allowed.is_empty() {
            return Ok(HashMap::new());
        }

        let query = EventsQuery {
            since: 0,
            limit: RECENT_EVENTS_LIMIT,
        };
        let mut events: Vec<SyncthingEvent> =
            self.get_json_with_query("/rest/events", &query).await?;
        events.sort_by(|a, b| b.id.cmp(&a.id));

        let mut changes: HashMap<String, Vec<FolderChange>> = HashMap::new();
        for event in events {
            if !is_file_event(&event.event_type) {
                continue;
            }
            let Some(folder_id) = event.folder_id() else {
                continue;
            };
            if !allowed.contains(folder_id) {
                continue;
            }
            if let Some(file_name) = event.file_name() {
                let entry = changes.entry(folder_id.to_string()).or_default();
                if !entry.is_empty() {
                    continue;
                }
                entry.push(FolderChange {
                    name: file_name,
                    action: event.action().unwrap_or_else(|| event.event_type.clone()),
                    when: format_relative_time(&event.time),
                    origin: event.origin(),
                });
            }
        }

        Ok(changes)
    }

    async fn get_json<T>(&mut self, path: &str) -> Result<T, MonitorError>
    where
        T: DeserializeOwned,
    {
        self.get_json_with_query(path, &()).await
    }

    async fn get_json_with_query<T, Q>(&mut self, path: &str, query: &Q) -> Result<T, MonitorError>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
    {
        let base = &self.base_urls[self.current_idx.min(self.base_urls.len().saturating_sub(1))];
        let url = format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .query(query)
            .send()
            .await
            .map_err(MonitorError::Http)?;

        if !response.status().is_success() {
            return Err(MonitorError::Syncthing(format!(
                "{} returned {}",
                path,
                response.status()
            )));
        }

        response.json::<T>().await.map_err(MonitorError::Http)
    }

    pub async fn get_gui_address(&mut self) -> Result<String, MonitorError> {
        let config: Value = self.get_json("/rest/config").await?;
        let address = config
            .get("gui")
            .and_then(|gui| gui.get("address"))
            .and_then(|addr| addr.as_str())
            .ok_or_else(|| {
                MonitorError::Syncthing("GUI address not found in config".to_string())
            })?;
        Ok(address.to_string())
    }

    pub async fn set_gui_address(&mut self, new_address: &str) -> Result<(), MonitorError> {
        // Get current config
        let mut config: Value = self.get_json("/rest/config").await?;

        // Update the GUI address
        if let Some(gui) = config.get_mut("gui") {
            if let Some(gui_obj) = gui.as_object_mut() {
                gui_obj.insert(
                    "address".to_string(),
                    Value::String(new_address.to_string()),
                );
            }
        }

        // Send the updated config back
        let base = &self.base_urls[self.current_idx.min(self.base_urls.len().saturating_sub(1))];
        let url = format!("{}/rest/config", base.trim_end_matches('/'));

        let response = self
            .http
            .put(url)
            .header("X-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&config)
            .send()
            .await
            .map_err(MonitorError::Http)?;

        if !response.status().is_success() {
            return Err(MonitorError::Syncthing(format!(
                "Failed to update GUI address: {}",
                response.status()
            )));
        }

        Ok(())
    }
}

fn push_unique_url(list: &mut Vec<String>, candidate: String) {
    if !list.iter().any(|existing| existing == &candidate) {
        list.push(candidate);
    }
}

#[derive(Serialize)]
struct FolderStatusQuery<'a> {
    folder: &'a str,
}

#[derive(Serialize)]
struct EventsQuery {
    since: u64,
    limit: u32,
}

#[derive(Serialize)]
struct EventStreamQuery<'a> {
    since: u64,
    limit: u32,
    timeout: u64,
    #[serde(rename = "events", skip_serializing_if = "Option::is_none")]
    events: Option<&'a [&'a str]>,
}

#[derive(Debug, Deserialize)]
struct SyncthingConfig {
    #[serde(default)]
    folders: Vec<FolderConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct FolderConfig {
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    paused: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SyncthingEvent {
    id: u64,
    #[serde(rename = "type")]
    event_type: String,
    time: String,
    data: Value,
}

impl SyncthingOverview {
    pub(crate) fn from_value(value: &Value) -> Self {
        Self {
            available: true,
            my_id: value
                .get("myID")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            version: value
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            state: value
                .get("state")
                .or_else(|| value.get("status"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            health: value
                .get("health")
                .or_else(|| value.get("status"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            started_at: value
                .get("startTime")
                .or_else(|| value.get("startedAt"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            uptime_seconds: value.get("uptime").and_then(|v| v.as_f64()),
            sequence: value
                .get("sequence")
                .or_else(|| value.get("dbSequence"))
                .and_then(|v| v.as_u64()),
            goroutine_count: value.get("goroutineCount").and_then(|v| v.as_u64()),
            errors: Vec::new(),
        }
    }

    pub(crate) fn error(message: String) -> Self {
        Self {
            errors: vec![message],
            ..Default::default()
        }
    }
}

impl FolderPayload {
    fn from_parts(folder: &FolderConfig, status: &Value, last_changes: Vec<FolderChange>) -> Self {
        let global_bytes = status.get("globalBytes").and_then(|v| v.as_u64());
        let need_bytes = status.get("needBytes").and_then(|v| v.as_u64());
        let in_sync_bytes = status.get("inSyncBytes").and_then(|v| v.as_u64());
        let completion = compute_completion(global_bytes, need_bytes);
        let state_raw = status
            .get("state")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let paused = folder.paused.unwrap_or(false);
        let state_info =
            humanize_folder_state(paused, state_raw.as_deref(), need_bytes, global_bytes);

        Self {
            id: folder.id.clone(),
            label: folder.label.clone().unwrap_or_else(|| folder.id.clone()),
            path: folder.path.clone(),
            state: state_info.label,
            state_code: state_info.code,
            state_raw,
            paused,
            global_bytes,
            in_sync_bytes,
            need_bytes,
            completion,
            last_changes,
        }
    }
}

impl SyncthingEvent {
    fn folder_id(&self) -> Option<&str> {
        self.data.get("folder").and_then(|v| v.as_str())
    }

    fn file_name(&self) -> Option<String> {
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

    fn action(&self) -> Option<String> {
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

    fn origin(&self) -> Option<String> {
        self.data
            .get("device")
            .or_else(|| self.data.get("peerID"))
            .or_else(|| self.data.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

fn is_file_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "ItemFinished"
            | "ItemStarted"
            | "LocalIndexUpdated"
            | "RemoteIndexUpdated"
            | "ItemDownloaded"
            | "FolderSummary"
            | "FolderCompletion"
    )
}

fn compute_completion(global_bytes: Option<u64>, need_bytes: Option<u64>) -> f64 {
    match (global_bytes, need_bytes) {
        (Some(global), Some(need)) if global > 0 => {
            let complete = global.saturating_sub(need);
            ((complete as f64 / global as f64) * 100.0).clamp(0.0, 100.0)
        }
        (Some(global), None) if global > 0 => 100.0,
        _ => 0.0,
    }
}

fn humanize_folder_state(
    paused: bool,
    state: Option<&str>,
    need_bytes: Option<u64>,
    global_bytes: Option<u64>,
) -> FolderStateInfo {
    if paused {
        return FolderStateInfo::new("Paused", FolderStateCode::Paused);
    }

    if let Some(state_value) = state {
        let normalized = state_value.to_ascii_lowercase();
        if normalized.contains("waiting") && normalized.contains("scan") {
            return FolderStateInfo::new("Waiting to scan", FolderStateCode::WaitingToScan);
        }
        if normalized.contains("waiting") && normalized.contains("sync") {
            return FolderStateInfo::new("Waiting to sync", FolderStateCode::WaitingToSync);
        }
        if normalized.contains("preparing") && normalized.contains("sync") {
            return FolderStateInfo::new("Preparing to sync", FolderStateCode::PreparingToSync);
        }

        if state_value.eq_ignore_ascii_case("scanning") {
            return FolderStateInfo::new("Scanning", FolderStateCode::Scanning);
        }
        if state_value.eq_ignore_ascii_case("syncing") {
            if let (Some(global), Some(need)) = (global_bytes, need_bytes) {
                if global > 0 && need > 0 {
                    let done = 100.0 - ((need as f64 / global as f64) * 100.0);
                    let label = format!("Syncing ({:.1}%)", done.clamp(0.0, 100.0));
                    return FolderStateInfo::new(label, FolderStateCode::Syncing);
                }
            }
            return FolderStateInfo::new("Syncing", FolderStateCode::Syncing);
        }
        if state_value.eq_ignore_ascii_case("idle") {
            if need_bytes.unwrap_or(0) == 0 {
                return FolderStateInfo::new("Up to date", FolderStateCode::UpToDate);
            }
            return FolderStateInfo::new("Idle / pending changes", FolderStateCode::PendingChanges);
        }
        if state_value.eq_ignore_ascii_case("error") {
            return FolderStateInfo::new("Error", FolderStateCode::Error);
        }
    }

    if need_bytes.unwrap_or(0) == 0 {
        FolderStateInfo::new("Up to date", FolderStateCode::UpToDate)
    } else {
        FolderStateInfo::new("Unknown state", FolderStateCode::Unknown)
    }
}

struct FolderStateInfo {
    label: String,
    code: FolderStateCode,
}

impl FolderStateInfo {
    fn new(label: impl Into<String>, code: FolderStateCode) -> Self {
        Self {
            label: label.into(),
            code,
        }
    }
}

fn format_relative_time(iso_time: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(iso_time) {
        Ok(parsed) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(parsed.with_timezone(&Utc));
            if duration.num_seconds() < 60 {
                "just now".to_string()
            } else if duration.num_minutes() < 60 {
                format!("{} min ago", duration.num_minutes())
            } else if duration.num_hours() < 24 {
                format!("{} h ago", duration.num_hours())
            } else {
                format!("{} d ago", duration.num_days())
            }
        }
        Err(_) => iso_time.to_string(),
    }
}

async fn load_api_key(config: &Config) -> Result<String, MonitorError> {
    if let Ok(value) = env::var("SYNCTHING_API_KEY") {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }

    let config_xml_path = config.syncthing_config_xml_path();
    let contents = fs::read_to_string(&config_xml_path)
        .await
        .map_err(|err| MonitorError::Io(err))?;
    extract_api_key(&contents).ok_or(MonitorError::MissingApiKey)
}

fn extract_api_key(contents: &str) -> Option<String> {
    let start_tag = "<apikey>";
    let end_tag = "</apikey>";
    let start = contents.find(start_tag)? + start_tag.len();
    let rest = &contents[start..];
    let end = rest.find(end_tag)?;
    Some(rest[..end].trim().to_string())
}
