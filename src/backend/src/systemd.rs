use std::collections::HashMap;

use tokio::process::Command;

use crate::config::Config;
use crate::types::{MonitorError, SystemdStatus};
use crate::ServiceAction;

pub async fn query_systemd_status(config: &Config) -> SystemdStatus {
    let service_name = &config.systemd_service_name;
    let mut status = SystemdStatus {
        name: service_name.to_string(),
        ..Default::default()
    };

    match Command::new("systemctl")
        .arg("show")
        .arg(service_name)
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
            .arg(service_name)
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

pub async fn control_syncthing_service(
    config: &Config,
    action: ServiceAction,
) -> Result<String, MonitorError> {
    let service_name = &config.systemd_service_name;
    let output = Command::new("systemctl")
        .arg(action.as_str())
        .arg(service_name)
        .output()
        .await?;

    if output.status.success() {
        Ok(format!("{} {}", service_name, action.past_tense()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(MonitorError::Systemd(if stderr.is_empty() {
            format!(
                "systemctl {} {} failed with status {}",
                action.as_str(),
                service_name,
                output.status
            )
        } else {
            format!(
                "systemctl {} {} failed: {}",
                action.as_str(),
                service_name,
                stderr
            )
        }))
    }
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
