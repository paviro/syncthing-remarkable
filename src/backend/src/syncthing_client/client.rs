use std::env;
use std::time::Duration;

use reqwest::Client;
use serde_json::Value;

use crate::config::Config;
use crate::types::MonitorError;

use super::api::{EventStreamQuery, EventWaitResult, SyncthingData, SyncthingEvent};
use super::core::{DataAggregator, HttpClient};
use super::helpers::load_api_key;

/// High-level client for interacting with the Syncthing REST API.
#[derive(Clone)]
pub struct SyncthingClient {
    http: HttpClient,
}

impl SyncthingClient {
    /// Discovers a Syncthing instance using config/env and prepares an HTTP client.
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

        let http_client = Client::builder()
            .timeout(Duration::from_secs(8))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(MonitorError::Http)?;

        Ok(Self {
            http: HttpClient::new(api_key, http_client, base_urls),
        })
    }

    /// Composes the full payload required by the UI.
    /// Fetches system status, config, recent changes and peer metrics.
    pub async fn compose_payload(&mut self) -> Result<SyncthingData, MonitorError> {
        let mut aggregator = DataAggregator::new(&mut self.http);
        aggregator.compose_payload().await
    }

    /// Long-polls the Syncthing event stream for updates.
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
        let events: Vec<SyncthingEvent> = self
            .http
            .get_json_with_query("/rest/events", &query)
            .await?;

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

    /// Fetches the GUI address from Syncthing configuration.
    pub async fn get_gui_address(&mut self) -> Result<String, MonitorError> {
        let config: Value = self.http.get_json("/rest/config").await?;
        let address = config
            .get("gui")
            .and_then(|gui| gui.get("address"))
            .and_then(|addr| addr.as_str())
            .ok_or_else(|| {
                MonitorError::Syncthing("GUI address not found in config".to_string())
            })?;
        Ok(address.to_string())
    }

    /// Updates the GUI address in Syncthing configuration.
    pub async fn set_gui_address(&mut self, new_address: &str) -> Result<(), MonitorError> {
        let mut config: Value = self.http.get_json("/rest/config").await?;

        // Update the GUI address
        if let Some(gui) = config.get_mut("gui") {
            if let Some(gui_obj) = gui.as_object_mut() {
                gui_obj.insert(
                    "address".to_string(),
                    Value::String(new_address.to_string()),
                );
            }
        }

        self.http.put_json("rest/config", &config).await
    }

    /// Restarts Syncthing via the API.
    /// Sends a POST request to /rest/system/restart which will cause Syncthing to restart itself.
    pub async fn restart(&mut self) -> Result<(), MonitorError> {
        self.http.post("/rest/system/restart").await
    }
}

/// Adds a URL to the list only if it's not already present.
fn push_unique_url(list: &mut Vec<String>, candidate: String) {
    if !list.iter().any(|existing| existing == &candidate) {
        list.push(candidate);
    }
}
