use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use tar::Archive;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::config::Config;
use crate::types::MonitorError;

const RELEASE_API_URL: &str = "https://api.github.com/repos/syncthing/syncthing/releases/latest";
const TARGET_ASSET_PREFIX: &str = "syncthing-linux-arm64-";
const TAR_EXTENSION: &str = ".tar.gz";
const INSTALLER_USER_AGENT: &str = "remarkable-syncthing-installer";

#[derive(Debug, Clone, Serialize, Default)]
pub struct InstallerStatus {
    pub binary_present: bool,
    pub service_installed: bool,
    pub in_progress: bool,
    pub progress_message: Option<String>,
    pub error: Option<String>,
    pub installer_disabled: bool,
}

pub struct Installer {
    config: Config,
    client: Client,
}

impl Installer {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .user_agent(INSTALLER_USER_AGENT)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { config, client }
    }

    pub async fn binary_present(&self) -> bool {
        match self.binary_path() {
            Ok(path) => fs::metadata(path)
                .await
                .map(|m| m.is_file())
                .unwrap_or(false),
            Err(err) => {
                eprintln!("Failed to resolve syncthing binary path: {err}");
                false
            }
        }
    }

    pub async fn service_installed(&self) -> bool {
        let service_name = &self.config.systemd_service_name;
        match Command::new("systemctl")
            .arg("cat")
            .arg(service_name)
            .output()
            .await
        {
            Ok(output) => output.status.success(),
            Err(err) => {
                eprintln!("Failed to query systemd unit {}: {err}", service_name);
                false
            }
        }
    }

    pub async fn download_latest_binary(&self) -> Result<(), MonitorError> {
        let asset = self.fetch_latest_asset().await?;
        let app_root = Config::app_root_dir()?;
        let tarball_path = app_root.join(&asset.name);
        self.download_asset(&asset.browser_download_url, &tarball_path)
            .await?;
        self.extract_binary(&tarball_path).await?;
        let _ = fs::remove_file(&tarball_path).await;
        Ok(())
    }

    pub async fn install_service(&self) -> Result<(), MonitorError> {
        self.remount_root_rw().await?;
        let service_result = self.install_service_inner().await;
        let restore_result = self.restore_mounts().await;

        if let Err(err) = &restore_result {
            eprintln!("Failed to restore mounts after installer run: {err}");
        }

        service_result.and(restore_result)
    }

    fn binary_path(&self) -> Result<PathBuf, MonitorError> {
        self.config.syncthing_binary_path()
    }

    async fn download_asset(&self, url: &str, destination: &Path) -> Result<(), MonitorError> {
        let mut response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()
            .map_err(|err| MonitorError::Http(err))?;

        let mut file = File::create(destination).await?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok(())
    }

    async fn extract_binary(&self, tarball_path: &Path) -> Result<(), MonitorError> {
        let binary_path = self.binary_path()?;
        let unpack_path = binary_path.clone();
        let tarball = tarball_path.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<(), MonitorError> {
            let file = std::fs::File::open(&tarball)?;
            let decoder = GzDecoder::new(file);
            let mut archive = Archive::new(decoder);
            let mut found = false;

            for entry_result in archive.entries()? {
                let mut entry = entry_result?;
                if entry
                    .path()?
                    .file_name()
                    .and_then(OsStr::to_str)
                    .map(|name| name == "syncthing")
                    .unwrap_or(false)
                {
                    entry.unpack(&unpack_path)?;
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(MonitorError::Config(
                    "Syncthing binary not found in downloaded archive".to_string(),
                ));
            }
            Ok(())
        })
        .await
        .map_err(|err| {
            MonitorError::Config(format!("Extraction task failed to complete: {err}"))
        })??;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o755);
            fs::set_permissions(&binary_path, permissions).await?;
        }

        Ok(())
    }

    async fn fetch_latest_asset(&self) -> Result<ReleaseAsset, MonitorError> {
        let response = self
            .client
            .get(RELEASE_API_URL)
            .send()
            .await?
            .error_for_status()
            .map_err(|err| MonitorError::Http(err))?;
        let release: Release = response.json().await?;
        release
            .assets
            .into_iter()
            .find(|asset| {
                asset.name.starts_with(TARGET_ASSET_PREFIX) && asset.name.ends_with(TAR_EXTENSION)
            })
            .ok_or_else(|| {
                MonitorError::Config(
                    "Latest Syncthing release does not contain the expected arm64 asset"
                        .to_string(),
                )
            })
    }

    async fn remount_root_rw(&self) -> Result<(), MonitorError> {
        run_command("mount", &["-o", "remount,rw", "/"]).await
    }

    async fn restore_mounts(&self) -> Result<(), MonitorError> {
        run_command("mount", &["-o", "remount,ro", "/"]).await?;
        Ok(())
    }

    async fn install_service_inner(&self) -> Result<(), MonitorError> {
        if let Err(err) = run_command("umount", &["-R", "/etc"]).await {
            if !is_umount_already_inactive(&err) {
                return Err(err);
            } else {
                eprintln!("Installer: /etc already unmounted, continuing");
            }
        }
        self.write_service_file().await?;
        run_command("systemctl", &["daemon-reload"]).await?;
        let service_name = &self.config.systemd_service_name;
        run_command("systemctl", &["enable", service_name]).await?;
        run_command("systemctl", &["start", service_name]).await
    }

    pub async fn restart_service(&self) -> Result<(), MonitorError> {
        let service_name = &self.config.systemd_service_name;
        run_command("systemctl", &["restart", service_name]).await
    }

    async fn write_service_file(&self) -> Result<(), MonitorError> {
        let unit_dir = Path::new("/etc/systemd/system");
        if !unit_dir.exists() {
            fs::create_dir_all(unit_dir).await?;
        }
        let unit_path = unit_dir.join(&self.config.systemd_service_name);
        let binary = self.binary_path()?;
        let contents = self.render_service_unit(&binary);
        fs::write(&unit_path, contents).await?;
        Ok(())
    }

    fn render_service_unit(&self, binary_path: &Path) -> String {
        format!(
            "[Unit]
Description=Syncthing
Documentation=man:syncthing(1)
After=network.target
StartLimitIntervalSec=60
StartLimitBurst=4

[Service]
User=root
WorkingDirectory=/home/root
Environment=HOME=/home/root
ExecStart={} serve --no-browser --no-restart --home={}
Restart=on-failure
RestartSec=5
SuccessExitStatus=3 4
RestartForceExitStatus=3 4

[Install]
WantedBy=multi-user.target
",
            binary_path.display(),
            self.config.syncthing_config_dir
        )
    }
}

#[derive(Debug, Deserialize)]
struct Release {
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

async fn run_command(command: &str, args: &[&str]) -> Result<(), MonitorError> {
    let output = Command::new(command).args(args).output().await?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(MonitorError::Config(if stderr.is_empty() {
        format!(
            "Command `{}` with args {:?} failed with status {}",
            command, args, output.status
        )
    } else {
        format!(
            "Command `{}` with args {:?} failed: {}",
            command, args, stderr
        )
    }))
}

fn is_umount_already_inactive(err: &MonitorError) -> bool {
    match err {
        MonitorError::Config(msg) => {
            msg.contains("not mounted")
                || msg.contains("not mounted.")
                || msg.contains("not mounted, cannot")
                || msg.contains("isn't mounted")
        }
        _ => false,
    }
}
