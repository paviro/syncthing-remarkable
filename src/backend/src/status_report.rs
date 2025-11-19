use chrono::{SecondsFormat, Utc};

use crate::config::Config;
use crate::syncthing_client::SyncthingClient;
use crate::systemd::query_systemd_status;
use crate::types::{MonitorError, StatusPayload, SyncthingOverview};

pub async fn build_status_payload(
    config: &Config,
    client_slot: &mut Option<SyncthingClient>,
    reason: &str,
) -> StatusPayload {
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let systemd = query_systemd_status(config).await;

    let (syncthing, folders) = match ensure_client(config, client_slot).await {
        Ok(client) => match client.compose_payload().await {
            Ok(payload) => (payload.overview, payload.folders),
            Err(err) => {
                eprintln!("collect payload failed: {err}");
                *client_slot = None;
                (SyncthingOverview::error(err.to_string()), Vec::new())
            }
        },
        Err(err) => (SyncthingOverview::error(err.to_string()), Vec::new()),
    };

    StatusPayload {
        fetched_at: timestamp,
        reason: reason.to_string(),
        systemd,
        syncthing,
        folders,
    }
}

async fn ensure_client<'a>(
    config: &Config,
    client_slot: &'a mut Option<SyncthingClient>,
) -> Result<&'a mut SyncthingClient, MonitorError> {
    if client_slot.is_none() {
        *client_slot = Some(SyncthingClient::discover(config).await?);
    }
    Ok(client_slot.as_mut().expect("client was just initialized"))
}
