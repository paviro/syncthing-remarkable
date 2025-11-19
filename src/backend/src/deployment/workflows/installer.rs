//! Installer for Syncthing binaries and systemd service setup.

use std::path::{Path, PathBuf};

use reqwest::Client;
use tokio::fs;
use tokio::process::Command;
use tracing::{error, warn};

use crate::deployment::system::architecture::detect_architecture;
use crate::deployment::system::archive;
use crate::config::Config;
use crate::deployment::http::assets::{self, ReleaseAsset};
use crate::deployment::http::client::{default_request_timeout, github_client};
use crate::deployment::http::download::download_to_path;
use crate::deployment::DownloadProgressSender;
use crate::utils::{filesystem, systemctl};
use crate::types::MonitorError;

const RELEASE_API_URL: &str = "https://api.github.com/repos/syncthing/syncthing/releases/latest";
const TAR_EXTENSION: &str = ".tar.gz";

#[derive(Clone)]
pub struct Installer {
    config: Config,
    client: Client,
}

impl Installer {
    pub fn new(config: Config) -> Self {
        let client = github_client(default_request_timeout())
            .expect("Failed to construct HTTP client for installer");
        Self { config, client }
    }

    pub async fn binary_present(&self) -> bool {
        match self.binary_path() {
            Ok(path) => fs::metadata(path)
                .await
                .map(|m| m.is_file())
                .unwrap_or(false),
            Err(err) => {
                error!(error = ?err, "Failed to resolve syncthing binary path");
                false
            }
        }
    }

    pub async fn service_installed(&self) -> bool {
        let service_name = &self.config.systemd_service_name;
        match Command::new("systemctl").arg("cat").arg(service_name).output().await {
            Ok(output) => output.status.success(),
            Err(err) => {
                error!(service = service_name, error = ?err, "Failed to query systemd unit");
                false
            }
        }
    }

    pub async fn download_latest_binary(
        &self,
        progress_tx: Option<DownloadProgressSender>,
    ) -> Result<(), MonitorError> {
        let asset = self.fetch_latest_asset().await?;
        let app_root = Config::app_root_dir()?;
        let tarball_path = app_root.join(&asset.name);

        download_to_path(
            &self.client,
            &asset.browser_download_url,
            &tarball_path,
            progress_tx,
        )
        .await?;

        self.extract_binary(&tarball_path).await?;
        let _ = fs::remove_file(&tarball_path).await;
        Ok(())
    }

    pub async fn install_service(&self) -> Result<(), MonitorError> {
        let was_readonly = filesystem::remount_root_rw().await?;
        let service_result = self.install_service_inner().await;
        let restore_result = filesystem::restore_mounts_if_needed(was_readonly).await;

        if let Err(err) = &restore_result {
            error!(error = ?err, "Failed to restore mounts after installer run");
        }

        service_result.and(restore_result)
    }

    pub async fn restart_service(&self) -> Result<(), MonitorError> {
        let service_name = &self.config.systemd_service_name;
        systemctl::execute(&["restart", service_name]).await
    }

    fn binary_path(&self) -> Result<PathBuf, MonitorError> {
        self.config.syncthing_binary_path()
    }

    async fn fetch_latest_asset(&self) -> Result<ReleaseAsset, MonitorError> {
        let architecture = detect_architecture().await?;
        let asset_prefix = architecture.syncthing_asset_prefix();
        let release = assets::fetch_release(&self.client, RELEASE_API_URL).await?;
        assets::select_asset_by_prefix(&release.assets, asset_prefix, TAR_EXTENSION)
            .cloned()
            .ok_or_else(|| {
                MonitorError::Config(format!(
                    "Latest Syncthing release does not contain the expected {} asset",
                    architecture.description()
                ))
            })
    }

    async fn extract_binary(&self, tarball_path: &Path) -> Result<(), MonitorError> {
        let binary_path = self.binary_path()?;
        archive::extract_tarball_entry(tarball_path, "syncthing", &binary_path).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o755);
            fs::set_permissions(&binary_path, permissions).await?;
        }

        Ok(())
    }

    async fn install_service_inner(&self) -> Result<(), MonitorError> {
        if let Err(err) = filesystem::unmount_etc_if_needed().await {
            warn!(error = ?err, "Warning during installer unmount");
        }
        self.write_service_file().await?;
        systemctl::execute(&["daemon-reload"]).await?;
        let service_name = &self.config.systemd_service_name;
        systemctl::execute(&["enable", service_name]).await?;
        systemctl::execute(&["start", service_name]).await
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

