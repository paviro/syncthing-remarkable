use tokio::process::Command;

use crate::types::MonitorError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    Arm32,
    Arm64,
}

impl Architecture {
    pub fn syncthing_asset_prefix(&self) -> &'static str {
        match self {
            Architecture::Arm32 => "syncthing-linux-arm-",
            Architecture::Arm64 => "syncthing-linux-arm64-",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Architecture::Arm32 => "arm (32-bit)",
            Architecture::Arm64 => "arm64 (64-bit)",
        }
    }

    fn from_machine_ident(ident: &str) -> Option<Self> {
        let normalized = ident.trim().to_lowercase();
        if normalized.is_empty() {
            return None;
        }

        match normalized.as_str() {
            "aarch64" | "arm64" => Some(Self::Arm64),
            "arm" | "armhf" => Some(Self::Arm32),
            value if value.starts_with("armv5") => Some(Self::Arm32),
            value if value.starts_with("armv6") => Some(Self::Arm32),
            value if value.starts_with("armv7") => Some(Self::Arm32),
            value if value.starts_with("armv8") => Some(Self::Arm64),
            value if value.contains("arm64") => Some(Self::Arm64),
            _ => None,
        }
    }
}

pub async fn detect_architecture() -> Result<Architecture, MonitorError> {
    let output = Command::new("uname").arg("-m").output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MonitorError::Config(format!(
            "Failed to detect system architecture: uname -m exited with status {} ({})",
            output.status,
            stderr.trim()
        )));
    }

    let machine = String::from_utf8_lossy(&output.stdout);
    Architecture::from_machine_ident(machine.trim()).ok_or_else(|| {
        MonitorError::Config(format!(
            "Unsupported system architecture reported by uname -m: {}",
            machine.trim()
        ))
    })
}

