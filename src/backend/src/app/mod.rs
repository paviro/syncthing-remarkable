mod event_stream;
mod operations;
pub mod protocol;
mod realtime;
mod status_builder;

pub use protocol::{ControlRequest, GuiAddressToggleRequest};

use appload_client::{AppLoadBackend, BackendReplier, Message};
use async_trait::async_trait;
use serde_json::json;
use tokio::task::JoinHandle;
use tracing::error;

use crate::config::Config;
use crate::deployment::{Installer, Updater};
use crate::syncthing_client::SyncthingClient;

use self::protocol::*;

pub struct Backend {
    pub client: Option<SyncthingClient>,
    pub config: Config,
    pub installer: Installer,
    pub install_in_progress: bool,
    pub install_progress_message: Option<String>,
    pub install_error: Option<String>,
    pub updater: Updater,
    pub update_in_progress: bool,
    pub update_progress_message: Option<String>,
    pub update_error: Option<String>,
    pub pending_update_url: Option<String>,
    pub update_pending_restart: bool,
    pub update_restart_seconds_remaining: Option<u32>,
    pub realtime_task: Option<JoinHandle<()>>,
    pub systemd_monitor_task: Option<JoinHandle<()>>,
}

impl Backend {
    pub async fn new(config: Config) -> Self {
        let client = SyncthingClient::discover(&config).await.ok();
        let installer = Installer::new(config.clone());
        let updater = Updater::new();
        Self {
            client,
            config,
            installer,
            install_in_progress: false,
            install_progress_message: None,
            install_error: None,
            updater,
            update_in_progress: false,
            update_progress_message: None,
            update_error: None,
            pending_update_url: None,
            update_pending_restart: false,
            update_restart_seconds_remaining: None,
            realtime_task: None,
            systemd_monitor_task: None,
        }
    }

    pub async fn send_status(&mut self, functionality: &BackendReplier<Self>, reason: &str) {
        let snapshot =
            status_builder::build_status_payload(&self.config, &mut self.client, reason).await;
        match serde_json::to_string(&snapshot) {
            Ok(payload) => {
                if let Err(err) = functionality.send_message(MSG_STATUS_UPDATE, &payload) {
                    error!(error = ?err, "Failed to send status update");
                }
            }
            Err(err) => self.send_error(functionality, &format!("Failed to encode payload: {err}")),
        }
    }

    pub fn send_error(&self, functionality: &BackendReplier<Self>, message: &str) {
        let payload = json!({ "message": message });
        if let Err(err) = functionality.send_message(MSG_ERROR, &payload.to_string()) {
            error!(error = ?err, "Failed to send error message");
        }
    }
}

#[async_trait]
impl AppLoadBackend for Backend {
    async fn handle_message(&mut self, functionality: &BackendReplier<Self>, message: Message) {
        match message.msg_type {
            MSG_SYSTEM_NEW_COORDINATOR => {
                self.ensure_realtime_updates(functionality);
                self.send_install_status(functionality).await;
                self.send_status(functionality, "frontend-connected").await;
            }
            MSG_CONTROL_REQUEST => {
                match serde_json::from_str::<ControlRequest>(&message.contents) {
                    Ok(req) => self.handle_systemd_control(functionality, req).await,
                    Err(err) => {
                        self.send_error(functionality, &format!("Invalid control payload: {err}"))
                    }
                }
            }
            MSG_INSTALL_TRIGGER => {
                if self.config.disable_syncthing_installer {
                    self.install_error = Some(
                        "Installer disabled via config. Please install Syncthing manually."
                            .to_string(),
                    );
                    self.install_progress_message = None;
                    self.install_in_progress = false;
                    self.send_install_status(functionality).await;
                } else if self.install_in_progress {
                    self.install_progress_message =
                        Some("Installer is already running...".to_string());
                    self.send_install_status(functionality).await;
                } else {
                    self.run_installer(functionality).await;
                }
            }
            MSG_GUI_ADDRESS_TOGGLE => {
                match serde_json::from_str::<GuiAddressToggleRequest>(&message.contents) {
                    Ok(req) => self.handle_syncthing_gui_listen_address(functionality, req).await,
                    Err(err) => self.send_error(
                        functionality,
                        &format!("Invalid GUI address toggle payload: {err}"),
                    ),
                }
            }
            MSG_UPDATE_CHECK_REQUEST => {
                self.handle_update_check(functionality).await;
            }
            MSG_UPDATE_DOWNLOAD_REQUEST => {
                self.handle_update_download(functionality).await;
            }
            MSG_UPDATE_RESTART_REQUEST => {
                self.handle_update_restart_request(functionality).await;
            }
            other => {
                self.send_error(functionality, &format!("Unknown message type {other}"));
            }
        }
    }
}

