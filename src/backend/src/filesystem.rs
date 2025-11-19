use tokio::process::Command;

use crate::types::MonitorError;

/// Check if root filesystem is mounted read-only
async fn is_root_readonly() -> Result<bool, MonitorError> {
    let output = Command::new("mount")
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(MonitorError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to query mount status"
        )));
    }
    
    let mount_output = String::from_utf8_lossy(&output.stdout);
    // Look for root mount and check if it has 'ro' option
    for line in mount_output.lines() {
        if line.contains(" on / ") || line.starts_with("/ ") {
            // Check if mount options contain 'ro'
            if let Some(opts_start) = line.rfind('(') {
                if let Some(opts_end) = line.rfind(')') {
                    let options = &line[opts_start+1..opts_end];
                    // Check for 'ro' as a standalone option (not part of another word)
                    for opt in options.split(',') {
                        if opt.trim() == "ro" {
                            return Ok(true);
                        }
                    }
                }
            }
        }
    }
    Ok(false)
}

/// Remount root filesystem as read-write, returns whether it was read-only before
pub async fn remount_root_rw() -> Result<bool, MonitorError> {
    let was_readonly = is_root_readonly().await?;
    
    if was_readonly {
        let output = Command::new("mount")
            .args(&["-o", "remount,rw", "/"])
            .output()
            .await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(MonitorError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to remount / as rw: {}", stderr)
            )));
        }
    }
    
    Ok(was_readonly)
}

/// Restore root filesystem to read-only and reapply /etc overlay (only if should_restore is true)
pub async fn restore_mounts_if_needed(should_restore: bool) -> Result<(), MonitorError> {
    if !should_restore {
        return Ok(());
    }
    
    {
        let output = Command::new("mount")
            .args(&["-o", "remount,ro", "/"])
            .output()
            .await;
        
        match output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                Err(MonitorError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to remount / as ro: {}", stderr)
                )))
            }
            Err(err) => Err(MonitorError::Io(err)),
        }
    }?;
    
    remount_etc_overlay().await
}

/// Reapply the /etc overlay mount used on reMarkable devices
async fn remount_etc_overlay() -> Result<(), MonitorError> {
    let output = Command::new("mount")
        .args(&[
            "-t",
            "overlay",
            "overlay",
            "-o",
            "lowerdir=/etc,upperdir=/var/volatile/etc,workdir=/var/volatile/.etc-work",
            "/etc",
        ])
        .output()
        .await;
    
    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            // If the overlay is already mounted, treat it as success
            if stderr.contains("busy") || stderr.contains("already mounted") {
                eprintln!("/etc overlay already mounted, skipping remount");
                return Ok(());
            }
            Err(MonitorError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to remount /etc overlay: {}", stderr)
            )))
        }
        Err(err) => Err(MonitorError::Io(err)),
    }
}

/// Unmount /etc overlay if needed (reMarkable specific)
pub async fn unmount_etc_if_needed() -> Result<(), MonitorError> {
    let output = Command::new("umount")
        .args(&["-R", "/etc"])
        .output()
        .await;
    
    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            // Check if already unmounted
            if stderr.contains("not mounted") || stderr.contains("isn't mounted") {
                eprintln!("/etc already unmounted, continuing");
                Ok(())
            } else {
                Err(MonitorError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to unmount /etc: {}", stderr)
                )))
            }
        }
        Err(err) => Err(MonitorError::Io(err)),
    }
}

