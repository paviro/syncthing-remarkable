use appload_client::BackendReplier;
use tokio::sync::mpsc;
use tracing::error;

use crate::deployment::{
    render_download_progress_message, should_emit_download_progress, InstallerStatus,
};
use crate::types::MonitorError;

use super::super::protocol::MSG_INSTALL_STATUS;
use super::super::Backend;

impl Backend {
    pub async fn send_install_status(&self, functionality: &BackendReplier<Self>) {
        if let Ok(payload) = serde_json::to_string(&self.build_install_status().await) {
            if let Err(err) = functionality.send_message(MSG_INSTALL_STATUS, &payload) {
                error!(error = ?err, "Failed to send installer status");
            }
        }
    }

    pub async fn build_install_status(&self) -> InstallerStatus {
        let binary_present = self.installer.binary_present().await;
        let service_installed = self.installer.service_installed().await;
        InstallerStatus {
            binary_present,
            service_installed,
            in_progress: self.install_in_progress,
            progress_message: self.install_progress_message.clone(),
            error: self.install_error.clone(),
            installer_disabled: self.config.disable_syncthing_installer,
        }
    }

    pub async fn run_installer(&mut self, functionality: &BackendReplier<Self>) {
        self.install_in_progress = true;
        self.install_error = None;
        self.install_progress_message = Some("Checking Syncthing installation...".to_string());
        self.send_install_status(functionality).await;

        if !self.installer.binary_present().await {
            self.install_progress_message =
                Some("Downloading latest Syncthing release...".to_string());
            self.send_install_status(functionality).await;
            let (progress_tx, mut progress_rx) = mpsc::channel(16);
            let installer = self.installer.clone();
            let mut download_future =
                Box::pin(installer.download_latest_binary(Some(progress_tx)));
            let mut download_result: Option<Result<(), MonitorError>> = None;
            let mut channel_open = true;
            let mut last_percent_reported: Option<u8> = None;
            let mut last_bytes_reported: u64 = 0;

            while download_result.is_none() || channel_open {
                tokio::select! {
                    result = &mut download_future, if download_result.is_none() => {
                        download_result = Some(result);
                    }
                    progress = progress_rx.recv(), if channel_open => {
                        match progress {
                            Some(progress) => {
                                if should_emit_download_progress(&progress, &mut last_percent_reported, &mut last_bytes_reported) {
                                    self.install_progress_message =
                                        Some(render_download_progress_message("Downloading latest Syncthing release", &progress));
                                    self.send_install_status(functionality).await;
                                }
                            }
                            None => channel_open = false,
                        }
                    }
                }
            }

            match download_result.unwrap() {
                Ok(()) => {}
                Err(err) => {
                    self.finish_installer_with_error(err, functionality).await;
                    return;
                }
            }
        }

        self.install_progress_message =
            Some("Binary ready. Preparing systemd service...".to_string());
        self.send_install_status(functionality).await;

        let service_installed = self.installer.service_installed().await;

        if !service_installed {
            self.install_progress_message =
                Some("Creating and enabling systemd service...".to_string());
            self.send_install_status(functionality).await;
            if let Err(err) = self.installer.install_service().await {
                self.finish_installer_with_error(err, functionality).await;
                return;
            }
        } else {
            self.install_progress_message =
                Some("Restarting existing Syncthing service...".to_string());
            self.send_install_status(functionality).await;
            if let Err(err) = self.installer.restart_service().await {
                self.finish_installer_with_error(err, functionality).await;
                return;
            }
        }

        self.install_progress_message = Some("Syncthing installed successfully.".to_string());
        self.install_in_progress = false;
        self.install_error = None;
        self.send_install_status(functionality).await;
        self.send_status(functionality, "installer").await;
    }

    pub async fn finish_installer_with_error(
        &mut self,
        err: MonitorError,
        functionality: &BackendReplier<Self>,
    ) {
        self.install_in_progress = false;
        self.install_error = Some(err.to_string());
        self.install_progress_message =
            Some("Installer failed. See error for details.".to_string());
        self.send_install_status(functionality).await;
    }
}

