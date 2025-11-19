//! Shared utilities for executing systemctl commands

use tokio::process::Command;

use crate::types::MonitorError;

/// Execute a systemctl command with the given arguments
/// Returns Ok(()) on success, or an error with stderr details on failure
pub async fn execute(args: &[&str]) -> Result<(), MonitorError> {
    let output = Command::new("systemctl").args(args).output().await?;
    
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(MonitorError::Systemd(if stderr.is_empty() {
        format!(
            "systemctl {} failed with status {}",
            args.join(" "),
            output.status
        )
    } else {
        format!("systemctl {} failed: {}", args.join(" "), stderr)
    }))
}

/// Execute a systemctl command with the given arguments
/// Returns a success message on success, or an error with stderr details on failure
pub async fn execute_with_message(args: &[&str], success_message: String) -> Result<String, MonitorError> {
    execute(args).await?;
    Ok(success_message)
}

