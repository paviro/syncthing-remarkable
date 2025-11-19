mod config;
mod installer;
mod status_report;
mod syncthing_client;
mod systemd;
mod types;
use appload_client::{
    AppLoad, AppLoadBackend, BackendReplier, Message, MSG_SYSTEM_NEW_COORDINATOR,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::Config;
use crate::installer::{Installer, InstallerStatus};
use crate::syncthing_client::SyncthingClient;
use crate::systemd::control_syncthing_service;
use crate::types::MonitorError;

const MSG_REFRESH_REQUEST: u32 = 1;
const MSG_CONTROL_REQUEST: u32 = 2;
const MSG_INSTALL_TRIGGER: u32 = 3;

const MSG_STATUS_UPDATE: u32 = 100;
const MSG_CONTROL_RESULT: u32 = 101;
const MSG_INSTALL_STATUS: u32 = 102;
const MSG_ERROR: u32 = 500;

#[tokio::main]
async fn main() {
    let config = Config::load().await;
    let backend = SyncthingBackend::new(config).await;
    match AppLoad::new(backend) {
        Ok(mut app) => {
            if let Err(err) = app.run().await {
                eprintln!("AppLoad backend exited with error: {err:?}");
            }
        }
        Err(err) => eprintln!("Failed to start AppLoad backend: {err:?}"),
    }
}

struct SyncthingBackend {
    client: Option<SyncthingClient>,
    config: Config,
    installer: Installer,
    install_in_progress: bool,
    install_progress_message: Option<String>,
    install_error: Option<String>,
}

impl SyncthingBackend {
    async fn new(config: Config) -> Self {
        let client = SyncthingClient::discover(&config).await.ok();
        let installer = Installer::new(config.clone());
        Self {
            client,
            config,
            installer,
            install_in_progress: false,
            install_progress_message: None,
            install_error: None,
        }
    }

    async fn send_status(&mut self, functionality: &BackendReplier<Self>, reason: &str) {
        let snapshot =
            status_report::build_status_payload(&self.config, &mut self.client, reason).await;
        match serde_json::to_string(&snapshot) {
            Ok(payload) => {
                if let Err(err) = functionality.send_message(MSG_STATUS_UPDATE, &payload) {
                    eprintln!("failed to send status: {err:?}");
                }
            }
            Err(err) => self.send_error(functionality, &format!("Failed to encode payload: {err}")),
        }
    }

    fn send_error(&self, functionality: &BackendReplier<Self>, message: &str) {
        let payload = json!({ "message": message });
        if let Err(err) = functionality.send_message(MSG_ERROR, &payload.to_string()) {
            eprintln!("failed to send error message: {err:?}");
        }
    }
}

#[async_trait]
impl AppLoadBackend for SyncthingBackend {
    async fn handle_message(&mut self, functionality: &BackendReplier<Self>, message: Message) {
        match message.msg_type {
            MSG_SYSTEM_NEW_COORDINATOR => {
                self.send_install_status(functionality).await;
                self.send_status(functionality, "frontend-connected").await;
            }
            MSG_REFRESH_REQUEST => {
                self.send_status(functionality, "manual").await;
            }
            MSG_CONTROL_REQUEST => {
                match serde_json::from_str::<ControlRequest>(&message.contents) {
                    Ok(req) => match control_syncthing_service(&self.config, req.action).await {
                        Ok(result) => {
                            let payload = json!({
                                "ok": true,
                                "action": req.action.as_str(),
                                "message": result
                            });
                            if let Err(err) =
                                functionality.send_message(MSG_CONTROL_RESULT, &payload.to_string())
                            {
                                eprintln!("failed to send control result: {err:?}");
                            }
                            self.send_status(functionality, "service-control").await;
                        }
                        Err(err) => {
                            let payload = json!({
                                "ok": false,
                                "action": req.action.as_str(),
                                "message": err.to_string()
                            });
                            if let Err(send_err) =
                                functionality.send_message(MSG_CONTROL_RESULT, &payload.to_string())
                            {
                                eprintln!("failed to send control error: {send_err:?}");
                            }
                        }
                    },
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
            other => {
                self.send_error(functionality, &format!("Unknown message type {other}"));
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ControlRequest {
    action: ServiceAction,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum ServiceAction {
    Start,
    Stop,
    Restart,
}

impl ServiceAction {
    fn as_str(&self) -> &'static str {
        match self {
            ServiceAction::Start => "start",
            ServiceAction::Stop => "stop",
            ServiceAction::Restart => "restart",
        }
    }

    fn past_tense(&self) -> &'static str {
        match self {
            ServiceAction::Start => "started",
            ServiceAction::Stop => "stopped",
            ServiceAction::Restart => "restarted",
        }
    }
}

impl SyncthingBackend {
    async fn send_install_status(&self, functionality: &BackendReplier<Self>) {
        if let Ok(payload) = serde_json::to_string(&self.build_install_status().await) {
            if let Err(err) = functionality.send_message(MSG_INSTALL_STATUS, &payload) {
                eprintln!("failed to send installer status: {err:?}");
            }
        }
    }

    async fn build_install_status(&self) -> InstallerStatus {
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

    async fn run_installer(&mut self, functionality: &BackendReplier<Self>) {
        self.install_in_progress = true;
        self.install_error = None;
        self.install_progress_message = Some("Checking Syncthing installation...".to_string());
        self.send_install_status(functionality).await;

        if !self.installer.binary_present().await {
            self.install_progress_message =
                Some("Downloading latest Syncthing release...".to_string());
            self.send_install_status(functionality).await;
            if let Err(err) = self.installer.download_latest_binary().await {
                self.finish_installer_with_error(err, functionality).await;
                return;
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

    async fn finish_installer_with_error(
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
