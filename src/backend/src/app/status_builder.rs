use chrono::{SecondsFormat, Utc};
use tracing::warn;

use crate::config::Config;
use crate::syncthing_client::SyncthingClient;
use crate::systemd::query_status;
use crate::types::{MonitorError, StatusPayload, SyncthingOverview};

/// Builds a complete status payload by aggregating data from multiple sources.
///
/// This orchestrates:
/// - SystemD service status
/// - Syncthing client initialization and data collection
/// - Error handling and fallback values
pub async fn build_status_payload(
    config: &Config,
    client_slot: &mut Option<SyncthingClient>,
    reason: &str,
) -> StatusPayload {
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let systemd = query_status(config).await;

    let (syncthing, folders, peers, gui_address) = match ensure_client(config, client_slot).await {
        Ok(client) => {
            let gui_addr = client.get_gui_address().await.ok();
            match client.compose_payload().await {
                Ok(payload) => (payload.overview, payload.folders, payload.peers, gui_addr),
                Err(err) => {
                    warn!(error = ?err, "Collecting payload failed");
                    *client_slot = None;
                    (
                        SyncthingOverview::error(err.to_string()),
                        Vec::new(),
                        Vec::new(),
                        None,
                    )
                }
            }
        }
        Err(err) => (
            SyncthingOverview::error(err.to_string()),
            Vec::new(),
            Vec::new(),
            None,
        ),
    };

    StatusPayload {
        fetched_at: timestamp,
        reason: reason.to_string(),
        systemd,
        syncthing,
        folders,
        peers,
        gui_address,
    }
}

/// Ensures the Syncthing client is initialized, creating it if necessary.
async fn ensure_client<'a>(
    config: &Config,
    client_slot: &'a mut Option<SyncthingClient>,
) -> Result<&'a mut SyncthingClient, MonitorError> {
    if client_slot.is_none() {
        *client_slot = Some(SyncthingClient::discover(config).await?);
    }
    Ok(client_slot.as_mut().expect("client was just initialized"))
}

