use appload_client::BackendReplier;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{error, warn};

use crate::deployment::{render_download_progress_message, should_emit_download_progress, UpdateStatus};
use crate::types::MonitorError;

use super::super::protocol::{MSG_UPDATE_CHECK_RESULT, MSG_UPDATE_DOWNLOAD_STATUS, UPDATE_RESTART_DELAY_SECS};
use super::super::Backend;

impl Backend {
    pub async fn handle_update_check(&mut self, functionality: &BackendReplier<Self>) {
        if self.update_in_progress {
            self.send_error(functionality, "Update already in progress");
            return;
        }

        self.update_in_progress = true;
        self.update_progress_message = Some("Checking for updates...".to_string());
        self.update_error = None;
        self.send_update_status(functionality).await;

        match self.updater.check_for_updates().await {
            Ok(result) => {
                self.pending_update_url = result.download_url.clone();
                self.update_in_progress = false;
                self.update_progress_message = None;

                if let Ok(payload) = serde_json::to_string(&result) {
                    if let Err(err) = functionality.send_message(MSG_UPDATE_CHECK_RESULT, &payload)
                    {
                        error!(error = ?err, "Failed to send update check result");
                    }
                }
                self.send_update_status(functionality).await;
            }
            Err(err) => {
                self.update_in_progress = false;
                self.update_error = Some(format!("Failed to check for updates: {}", err));
                self.update_progress_message = None;
                self.send_update_status(functionality).await;
            }
        }
    }

    pub async fn handle_update_download(&mut self, functionality: &BackendReplier<Self>) {
        if self.update_in_progress {
            self.send_error(functionality, "Update already in progress");
            return;
        }

        let download_url = match &self.pending_update_url {
            Some(url) => url.clone(),
            None => {
                self.send_error(functionality, "No update available to download");
                return;
            }
        };

        self.update_in_progress = true;
        self.update_error = None;
        self.update_progress_message = Some("Downloading update...".to_string());
        self.update_pending_restart = false;
        self.update_restart_seconds_remaining = None;
        self.send_update_status(functionality).await;

        let (progress_tx, mut progress_rx) = mpsc::channel(16);
        let updater = self.updater.clone();
        let mut update_future = Box::pin(
            updater.download_and_apply_update(&download_url, Some(progress_tx)),
        );
        let mut update_result: Option<Result<(), MonitorError>> = None;
        let mut channel_open = true;
        let mut download_phase_reported_complete = false;
        let mut last_percent_reported: Option<u8> = None;
        let mut last_bytes_reported: u64 = 0;

        while update_result.is_none() || channel_open {
            tokio::select! {
                result = &mut update_future, if update_result.is_none() => {
                    update_result = Some(result);
                }
                progress = progress_rx.recv(), if channel_open => {
                    match progress {
                        Some(progress) => {
                            if should_emit_download_progress(&progress, &mut last_percent_reported, &mut last_bytes_reported) {
                                self.update_progress_message =
                                    Some(render_download_progress_message("Downloading update", &progress));
                                self.send_update_status(functionality).await;
                            }
                        }
                        None => {
                            channel_open = false;
                            if !download_phase_reported_complete && update_result.is_none() {
                                download_phase_reported_complete = true;
                                self.update_progress_message =
                                    Some("Installing update files...".to_string());
                                self.send_update_status(functionality).await;
                            }
                        }
                    }
                }
            }
        }

        match update_result.unwrap() {
            Ok(()) => {
                self.begin_restart_countdown(functionality).await;
            }
            Err(err) => {
                self.update_in_progress = false;
                self.update_error = Some(format!("Failed to download/apply update: {}", err));
                self.update_progress_message = None;
                self.update_pending_restart = false;
                self.update_restart_seconds_remaining = None;
                self.send_update_status(functionality).await;
            }
        }
    }

    pub async fn send_update_status(&self, functionality: &BackendReplier<Self>) {
        let status = UpdateStatus {
            in_progress: self.update_in_progress,
            progress_message: self.update_progress_message.clone(),
            error: self.update_error.clone(),
            success: !self.update_in_progress && self.update_error.is_none(),
            pending_restart: self.update_pending_restart,
            restart_seconds_remaining: self.update_restart_seconds_remaining,
        };

        if let Ok(payload) = serde_json::to_string(&status) {
            if let Err(err) = functionality.send_message(MSG_UPDATE_DOWNLOAD_STATUS, &payload) {
                error!(error = ?err, "Failed to send update status");
            }
        }
    }

    pub async fn begin_restart_countdown(&mut self, functionality: &BackendReplier<Self>) {
        self.update_in_progress = false;
        self.update_error = None;
        self.pending_update_url = None;
        self.update_pending_restart = true;
        self.update_restart_seconds_remaining = Some(UPDATE_RESTART_DELAY_SECS as u32);
        self.update_progress_message = Some("Update installed. Restarting shortly...".to_string());
        self.send_update_status(functionality).await;
        self.schedule_delayed_restart();
    }

    pub fn schedule_delayed_restart(&self) {
        tokio::spawn(async {
            sleep(Duration::from_secs(UPDATE_RESTART_DELAY_SECS)).await;
            warn!("Restarting backend after update countdown finished...");
            std::process::exit(0);
        });
    }

    pub async fn handle_update_restart_request(&mut self, functionality: &BackendReplier<Self>) {
        if !self.update_pending_restart {
            self.send_error(functionality, "No pending update closure");
            return;
        }
        self.update_progress_message = Some("Restarting now...".to_string());
        self.update_restart_seconds_remaining = Some(0);
        self.send_update_status(functionality).await;
        sleep(Duration::from_millis(250)).await;
        std::process::exit(0);
    }
}

