use tokio::time::Duration;

use super::client::query_status;
use super::types::SystemdStatus;
use crate::config::Config;

/// Check if systemd state has changed between two status snapshots
pub fn state_changed(previous: &SystemdStatus, current: &SystemdStatus) -> bool {
    previous.active_state != current.active_state
        || previous.sub_state != current.sub_state
        || previous.result != current.result
        || previous.unit_file_state != current.unit_file_state
        || previous.pid != current.pid
}

/// Monitor a systemd service continuously
/// This function runs continuously and polls systemd status at regular intervals
pub async fn monitor_service<F>(config: Config, interval_secs: u64, mut on_change: F)
where
    F: FnMut() + Send + 'static,
{
    let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
    let mut last_status: Option<SystemdStatus> = None;

    loop {
        ticker.tick().await;
        let status = query_status(&config).await;
        let changed = match &last_status {
            None => true,
            Some(previous) => state_changed(previous, &status),
        };

        if changed {
            on_change();
        }

        last_status = Some(status);
    }
}

