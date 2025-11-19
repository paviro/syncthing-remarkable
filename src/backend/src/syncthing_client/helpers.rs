use chrono::Utc;
use std::env;
use tokio::fs;

use crate::config::Config;
use crate::types::MonitorError;

use super::types::{FolderStateCode, FolderStateInfo};

pub const RECENT_EVENTS_LIMIT: u32 = 200;

pub fn is_file_event(event_type: &str) -> bool {
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

pub fn compute_completion(global_bytes: Option<u64>, need_bytes: Option<u64>) -> f64 {
    match (global_bytes, need_bytes) {
        (Some(global), Some(need)) if global > 0 => {
            let complete = global.saturating_sub(need);
            ((complete as f64 / global as f64) * 100.0).clamp(0.0, 100.0)
        }
        (Some(global), None) if global > 0 => 100.0,
        _ => 0.0,
    }
}

pub fn humanize_folder_state(
    paused: bool,
    state: Option<&str>,
    need_bytes: Option<u64>,
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

pub fn format_relative_time(iso_time: &str) -> String {
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

pub async fn load_api_key(config: &Config) -> Result<String, MonitorError> {
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

