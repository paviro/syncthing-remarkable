use appload_client::BackendReplier;
use tokio::time::{sleep, Duration, Instant};
use tracing::warn;

use crate::config::Config;
use crate::syncthing_client::SyncthingClient;

use super::protocol::{EVENT_HEARTBEAT_SECS, EVENT_RECONNECT_DELAY_SECS, EVENT_STREAM_TIMEOUT_SECS};
use super::Backend;

/// Drives the Syncthing event stream and orchestrates when to send status updates.
///
/// This implements:
/// - Event polling with automatic reconnection
/// - Heartbeat timing (send status even without events)
/// - Backend notification decisions
pub async fn drive_syncthing_stream(
    functionality: BackendReplier<Backend>,
    config: Config,
) {
    let mut client: Option<SyncthingClient> = None;
    let mut last_event_id: u64 = 0;
    let mut last_emit = Instant::now() - Duration::from_secs(EVENT_HEARTBEAT_SECS);

    loop {
        // Ensure client is connected
        if client.is_none() {
            match SyncthingClient::discover(&config).await {
                Ok(new_client) => {
                    client = Some(new_client);
                    last_event_id = 0;
                }
                Err(err) => {
                    warn!(error = ?err, "Failed to connect to Syncthing");
                    sleep(Duration::from_secs(EVENT_RECONNECT_DELAY_SECS)).await;
                    continue;
                }
            }
        }

        // Poll for events
        let timeout = Duration::from_secs(EVENT_STREAM_TIMEOUT_SECS);
        let wait_result = client
            .as_mut()
            .expect("client initialized")
            .wait_for_updates(last_event_id, timeout)
            .await;

        match wait_result {
            Ok(result) => {
                last_event_id = result.last_event_id;
                let has_events = result.has_updates;
                let heartbeat_due = last_emit.elapsed().as_secs() >= EVENT_HEARTBEAT_SECS;

                if has_events || heartbeat_due {
                    let reason = if has_events {
                        "syncthing-event"
                    } else {
                        "syncthing-heartbeat"
                    };

                    let mut backend = functionality.backend.lock().await;
                    backend.send_status(&functionality, reason).await;
                    last_emit = Instant::now();
                }
            }
            Err(err) => {
                warn!(error = ?err, "Syncthing event polling error");
                client = None; // Force reconnection
                sleep(Duration::from_secs(EVENT_RECONNECT_DELAY_SECS)).await;
            }
        }
    }
}

