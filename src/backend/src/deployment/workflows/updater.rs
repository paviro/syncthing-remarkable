//! Updater for the Syncthing-for-reMarkable AppLoad bundle.

use std::path::{Path, PathBuf};

use reqwest::Client;
use serde_json::Value;
use tempfile::TempDir;
use tokio::fs;
use tracing::info;

use crate::deployment::system::architecture::{detect_architecture, Architecture};
use crate::deployment::system::archive;
use crate::config::Config;
use crate::deployment::http::assets;
use crate::deployment::http::client::{default_request_timeout, github_client};
use crate::deployment::http::download::download_to_path;
use crate::deployment::{DownloadProgressSender, UpdateCheckResult};
use crate::types::MonitorError;

const RELEASE_API_URL: &str =
    "https://api.github.com/repos/paviro/Syncthing-for-reMarkable/releases/latest";

#[derive(Clone)]
pub struct Updater {
    client: Client,
}

impl Updater {
    pub fn new() -> Self {
        let client = github_client(default_request_timeout())
            .expect("Failed to construct HTTP client for updater");
        Self { client }
    }

    pub async fn get_current_version() -> Result<String, MonitorError> {
        let manifest_path = Self::get_manifest_path()?;
        let contents = fs::read_to_string(&manifest_path).await?;
        let manifest: Value = serde_json::from_str(&contents)?;

        manifest
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                MonitorError::Config("Version field not found in manifest.json".to_string())
            })
    }

    fn get_manifest_path() -> Result<PathBuf, MonitorError> {
        let app_root = Config::app_root_dir()?;
        Ok(app_root.join("manifest.json"))
    }

    pub async fn check_for_updates(&self) -> Result<UpdateCheckResult, MonitorError> {
        let current_version = Self::get_current_version().await?;
        let architecture = detect_architecture().await?;
        let release = assets::fetch_release(&self.client, RELEASE_API_URL).await?;
        let latest_version = release.tag_name.trim_start_matches('v').to_string();
        let update_available = self.compare_versions(&current_version, &latest_version)?;

        let download_url = if update_available {
            let asset_name = self.get_asset_name_for_arch(architecture);
            assets::select_asset_exact(&release.assets, &asset_name)
                .map(|asset| asset.browser_download_url.clone())
        } else {
            None
        };

        Ok(UpdateCheckResult {
            current_version,
            latest_version,
            update_available,
            download_url,
        })
    }

    fn compare_versions(&self, current: &str, latest: &str) -> Result<bool, MonitorError> {
        let current_semver = semver::Version::parse(current).map_err(|err| {
            MonitorError::Config(format!("Invalid current version '{}': {}", current, err))
        })?;

        let latest_semver = semver::Version::parse(latest).map_err(|err| {
            MonitorError::Config(format!("Invalid latest version '{}': {}", latest, err))
        })?;

        Ok(latest_semver > current_semver)
    }

    fn get_asset_name_for_arch(&self, arch: Architecture) -> String {
        match arch {
            Architecture::Arm64 => "syncthing-rm-appload-aarch64.zip".to_string(),
            Architecture::Arm32 => "syncthing-rm-appload-armv7.zip".to_string(),
        }
    }

    pub async fn download_and_apply_update(
        &self,
        download_url: &str,
        progress_tx: Option<DownloadProgressSender>,
    ) -> Result<(), MonitorError> {
        let temp_dir = TempDir::new().map_err(|err| {
            MonitorError::Config(format!("Failed to create temporary directory: {}", err))
        })?;

        let zip_path = temp_dir.path().join("update.zip");
        download_to_path(&self.client, download_url, &zip_path, progress_tx).await?;

        let extract_dir = temp_dir.path().join("extracted");
        fs::create_dir_all(&extract_dir).await?;
        archive::extract_zip_archive(&zip_path, &extract_dir).await?;

        let app_root = Config::app_root_dir()?;
        self.copy_update_files(&extract_dir, &app_root).await
    }

    async fn copy_update_files(
        &self,
        source_dir: &Path,
        dest_dir: &Path,
    ) -> Result<(), MonitorError> {
        let payload_root = self.resolve_payload_root(source_dir).await?;
        let mut entries = fs::read_dir(&payload_root).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if Self::should_skip_entry(&file_name_str) {
                info!(entry = %file_name_str, "Skipping update entry");
                continue;
            }

            let path = entry.path();
            let dest_path = dest_dir.join(&file_name);
            let file_type = entry.file_type().await?;

            if file_type.is_dir() {
                self.copy_dir_recursive(&path, &dest_path).await?;
            } else if file_type.is_file() {
                self.copy_file_atomic(&path, &dest_path).await?;
            }
        }

        Ok(())
    }

    async fn resolve_payload_root(&self, extracted_dir: &Path) -> Result<PathBuf, MonitorError> {
        let manifest_at_root = extracted_dir.join("manifest.json");
        if fs::metadata(&manifest_at_root).await.is_ok() {
            return Ok(extracted_dir.to_path_buf());
        }

        let mut entries = fs::read_dir(extracted_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if name.starts_with("__MACOSX") {
                continue;
            }

            let file_type = entry.file_type().await?;
            if file_type.is_dir() {
                let candidate = entry.path();
                if fs::metadata(candidate.join("manifest.json")).await.is_ok() {
                    return Ok(candidate);
                }
            }
        }

        Err(MonitorError::Config(
            "Downloaded update did not contain a manifest.json".to_string(),
        ))
    }

    fn should_skip_entry(name: &str) -> bool {
        name == "config.json"
            || name == "syncthing"
            || name.starts_with("__MACOSX")
            || name.starts_with("._")
            || name == ".DS_Store"
    }

    fn copy_dir_recursive<'a>(
        &'a self,
        source: &'a Path,
        dest: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), MonitorError>> + Send + 'a>>
    {
        Box::pin(async move {
            if fs::metadata(dest).await.is_err() {
                fs::create_dir_all(dest).await?;
            }

            let mut entries = fs::read_dir(source).await?;

            while let Some(entry) = entries.next_entry().await? {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                if Self::should_skip_entry(&file_name_str) {
                    continue;
                }

                let path = entry.path();
                let dest_path = dest.join(&file_name);
                let file_type = entry.file_type().await?;

                match file_type {
                    ft if ft.is_dir() => {
                        self.copy_dir_recursive(&path, &dest_path).await?;
                    }
                    ft if ft.is_file() => {
                        self.copy_file_atomic(&path, &dest_path).await?;
                    }
                    _ => {}
                }
            }

            Ok(())
        })
    }

    async fn copy_file_atomic(&self, source: &Path, dest: &Path) -> Result<(), MonitorError> {
        if let Some(parent) = dest.parent() {
            if fs::metadata(parent).await.is_err() {
                fs::create_dir_all(parent).await?;
            }
        }

        let tmp_path = self.temp_path_for(dest);
        fs::copy(source, &tmp_path).await?;
        match fs::rename(&tmp_path, dest).await {
            Ok(_) => {
                info!(path = %dest.display(), "Updated file");
                Ok(())
            }
            Err(err) => {
                let _ = fs::remove_file(&tmp_path).await;
                Err(MonitorError::Io(err))
            }
        }
    }

    fn temp_path_for(&self, dest: &Path) -> PathBuf {
        let file_name = dest
            .file_name()
            .and_then(|n| n.to_str())
            .map(|name| format!("{}.tmp-update", name))
            .unwrap_or_else(|| "tmp-update-file".to_string());
        dest.with_file_name(file_name)
    }
}

