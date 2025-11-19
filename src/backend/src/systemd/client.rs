use std::collections::HashMap;

use tokio::process::Command;
use tracing::{error, warn};

use crate::config::Config;
use crate::filesystem;
use crate::systemctl;
use crate::types::MonitorError;

use super::actions::ServiceAction;
use super::types::SystemdStatus;

pub struct SystemdClient<'a> {
    service_name: &'a str,
}

impl<'a> SystemdClient<'a> {
    pub fn new(service_name: &'a str) -> Self {
        Self { service_name }
    }

    /// Query the current status of the systemd service
    pub async fn query_status(&self) -> SystemdStatus {
        let mut status = SystemdStatus {
            name: self.service_name.to_string(),
            ..Default::default()
        };

        match Command::new("systemctl")
            .arg("show")
            .arg(self.service_name)
            .arg("--no-page")
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                if let Ok(map) = parse_systemctl_show(&output.stdout) {
                    status.active_state = map.get("ActiveState").cloned();
                    status.sub_state = map.get("SubState").cloned();
                    status.unit_file_state = map.get("UnitFileState").cloned();
                    status.result = map.get("Result").cloned();
                    status.description = map.get("Description").cloned();
                    status.active_enter_timestamp = map.get("ActiveEnterTimestamp").cloned();
                    status.inactive_enter_timestamp = map.get("InactiveEnterTimestamp").cloned();
                    status.pid = map
                        .get("ExecMainPID")
                        .and_then(|pid| pid.parse::<u32>().ok());
                }
            }
            Ok(output) => {
                status.error = Some(format!("systemctl show failed: {}", output.status));
                if !output.stderr.is_empty() {
                    status.raw_excerpt =
                        Some(String::from_utf8_lossy(&output.stderr).trim().to_string());
                }
            }
            Err(err) => {
                status.error = Some(format!("systemctl show error: {err}"));
            }
        }

        if status.raw_excerpt.is_none() {
            if let Ok(output) = Command::new("systemctl")
                .arg("status")
                .arg(self.service_name)
                .arg("--no-pager")
                .arg("--lines=5")
                .output()
                .await
            {
                if output.status.success() {
                    status.raw_excerpt =
                        Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
            }
        }

        status
    }

    /// Control the systemd service (start, stop, restart, enable, disable)
    pub async fn control_service(&self, action: ServiceAction) -> Result<String, MonitorError> {
        let success_message = format!("{} {}", self.service_name, action.past_tense());
        
        if action.needs_remount() {
            // Remount filesystem as read-write, track if it was read-only before
            let was_readonly = filesystem::remount_root_rw().await?;

            // Try to unmount /etc overlay if it exists
            if let Err(err) = filesystem::unmount_etc_if_needed().await {
                warn!(error = ?err, "Failed to unmount /etc before systemctl operation");
                // Continue anyway, might not be critical
            }

            // Execute the systemctl command
            let result = systemctl::execute_with_message(
                &[action.as_str(), self.service_name],
                success_message,
            )
            .await;

            // Restore read-only mount only if it was read-only before
            if let Err(restore_err) = filesystem::restore_mounts_if_needed(was_readonly).await {
                error!(error = ?restore_err, "Failed to restore mounts after systemctl operation");
            }

            result
        } else {
            systemctl::execute_with_message(&[action.as_str(), self.service_name], success_message).await
        }
    }
}

/// Convenience function to query systemd status using config
pub async fn query_status(config: &Config) -> SystemdStatus {
    let client = SystemdClient::new(&config.systemd_service_name);
    client.query_status().await
}

/// Convenience function to control systemd service using config
pub async fn control_service(
    config: &Config,
    action: ServiceAction,
) -> Result<String, MonitorError> {
    let client = SystemdClient::new(&config.systemd_service_name);
    client.control_service(action).await
}

fn parse_systemctl_show(bytes: &[u8]) -> Result<HashMap<String, String>, MonitorError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|err| MonitorError::Systemd(format!("Invalid UTF-8 from systemctl: {err}")))?;
    let mut map = HashMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(map)
}

