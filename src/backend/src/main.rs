mod architecture;
mod archive;
mod config;
mod filesystem;
mod installer;
mod status_report;
mod syncthing_client;
mod systemd;
mod types;
mod updater;
use appload_client::{
    AppLoad, AppLoadBackend, BackendReplier, Message, MSG_SYSTEM_NEW_COORDINATOR,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration, Instant};
use tracing::{error, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::installer::{Installer, InstallerStatus};
use crate::syncthing_client::SyncthingClient;
use crate::systemd::{control_syncthing_service, query_systemd_status};
use crate::types::{MonitorError, SystemdStatus};
use crate::updater::{UpdateStatus, Updater};

const MSG_CONTROL_REQUEST: u32 = 2;
const MSG_INSTALL_TRIGGER: u32 = 3;
const MSG_GUI_ADDRESS_TOGGLE: u32 = 4;
const MSG_UPDATE_CHECK_REQUEST: u32 = 5;
const MSG_UPDATE_DOWNLOAD_REQUEST: u32 = 6;
const MSG_UPDATE_RESTART_REQUEST: u32 = 7;

const MSG_STATUS_UPDATE: u32 = 100;
const MSG_CONTROL_RESULT: u32 = 101;
const MSG_INSTALL_STATUS: u32 = 102;
const MSG_GUI_ADDRESS_RESULT: u32 = 103;
const MSG_UPDATE_CHECK_RESULT: u32 = 104;
const MSG_UPDATE_DOWNLOAD_STATUS: u32 = 105;
const MSG_ERROR: u32 = 500;

const UPDATE_RESTART_DELAY_SECS: u64 = 10;
const EVENT_STREAM_TIMEOUT_SECS: u64 = 30;
const EVENT_HEARTBEAT_SECS: u64 = 5;
const EVENT_RECONNECT_DELAY_SECS: u64 = 5;
const SYSTEMD_MONITOR_INTERVAL_SECS: u64 = 5;

#[tokio::main]
async fn main() {
    init_tracing();
    let config = Config::load().await;
    let backend = SyncthingBackend::new(config).await;
    match AppLoad::new(backend) {
        Ok(mut app) => {
            if let Err(err) = app.run().await {
                error!(error = ?err, "AppLoad backend exited with error");
            }
        }
        Err(err) => error!(error = ?err, "Failed to start AppLoad backend"),
    }
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();
}

struct SyncthingBackend {
    client: Option<SyncthingClient>,
    config: Config,
    installer: Installer,
    install_in_progress: bool,
    install_progress_message: Option<String>,
    install_error: Option<String>,
    updater: Updater,
    update_in_progress: bool,
    update_progress_message: Option<String>,
    update_error: Option<String>,
    pending_update_url: Option<String>,
    update_pending_restart: bool,
    update_restart_seconds_remaining: Option<u32>,
    realtime_task: Option<JoinHandle<()>>,
    systemd_monitor_task: Option<JoinHandle<()>>,
}

impl SyncthingBackend {
    async fn new(config: Config) -> Self {
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

    async fn send_status(&mut self, functionality: &BackendReplier<Self>, reason: &str) {
        let snapshot =
            status_report::build_status_payload(&self.config, &mut self.client, reason).await;
        match serde_json::to_string(&snapshot) {
            Ok(payload) => {
                if let Err(err) = functionality.send_message(MSG_STATUS_UPDATE, &payload) {
                    error!(error = ?err, "Failed to send status update");
                }
            }
            Err(err) => self.send_error(functionality, &format!("Failed to encode payload: {err}")),
        }
    }

    fn send_error(&self, functionality: &BackendReplier<Self>, message: &str) {
        let payload = json!({ "message": message });
        if let Err(err) = functionality.send_message(MSG_ERROR, &payload.to_string()) {
            error!(error = ?err, "Failed to send error message");
        }
    }
}

fn task_is_running(handle: &Option<JoinHandle<()>>) -> bool {
    handle
        .as_ref()
        .map(|handle| !handle.is_finished())
        .unwrap_or(false)
}

#[async_trait]
impl AppLoadBackend for SyncthingBackend {
    async fn handle_message(&mut self, functionality: &BackendReplier<Self>, message: Message) {
        match message.msg_type {
            MSG_SYSTEM_NEW_COORDINATOR => {
                self.ensure_realtime_updates(functionality);
                self.send_install_status(functionality).await;
                self.send_status(functionality, "frontend-connected").await;
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
                                error!(error = ?err, "Failed to send control result");
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
                                error!(error = ?send_err, "Failed to send control error response");
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
            MSG_GUI_ADDRESS_TOGGLE => {
                match serde_json::from_str::<GuiAddressToggleRequest>(&message.contents) {
                    Ok(req) => {
                        if let Some(client) = &mut self.client {
                            match client.set_gui_address(&req.address).await {
                                Ok(()) => {
                                    let payload = json!({
                                        "ok": true,
                                        "address": req.address,
                                        "message": format!("GUI address updated to {}", req.address)
                                    });
                                    if let Err(err) = functionality
                                        .send_message(MSG_GUI_ADDRESS_RESULT, &payload.to_string())
                                    {
                                        error!(error = ?err, "Failed to send GUI address result");
                                    }
                                    self.send_status(functionality, "gui-address-change").await;
                                }
                                Err(err) => {
                                    let payload = json!({
                                        "ok": false,
                                        "address": req.address,
                                        "message": format!("Failed to update GUI address: {}", err)
                                    });
                                    if let Err(send_err) = functionality
                                        .send_message(MSG_GUI_ADDRESS_RESULT, &payload.to_string())
                                    {
                                        error!(error = ?send_err, "Failed to send GUI address error");
                                    }
                                }
                            }
                        } else {
                            self.send_error(functionality, "Syncthing client not available");
                        }
                    }
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

#[derive(Debug, Deserialize)]
struct ControlRequest {
    action: ServiceAction,
}

#[derive(Debug, Deserialize)]
struct GuiAddressToggleRequest {
    address: String,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ServiceAction {
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
}

impl ServiceAction {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ServiceAction::Start => "start",
            ServiceAction::Stop => "stop",
            ServiceAction::Restart => "restart",
            ServiceAction::Enable => "enable",
            ServiceAction::Disable => "disable",
        }
    }

    pub(crate) fn past_tense(&self) -> &'static str {
        match self {
            ServiceAction::Start => "started",
            ServiceAction::Stop => "stopped",
            ServiceAction::Restart => "restarted",
            ServiceAction::Enable => "enabled",
            ServiceAction::Disable => "disabled",
        }
    }
}

impl SyncthingBackend {
    fn ensure_realtime_updates(&mut self, functionality: &BackendReplier<Self>) {
        if !task_is_running(&self.realtime_task) {
            let config = self.config.clone();
            let replier = functionality.clone();
            self.realtime_task = Some(tokio::spawn(async move {
                drive_syncthing_stream(replier, config).await;
            }));
        }

        if !task_is_running(&self.systemd_monitor_task) {
            let config = self.config.clone();
            let replier = functionality.clone();
            self.systemd_monitor_task = Some(tokio::spawn(async move {
                drive_systemd_monitor(replier, config).await;
            }));
        }
    }

    async fn send_install_status(&self, functionality: &BackendReplier<Self>) {
        if let Ok(payload) = serde_json::to_string(&self.build_install_status().await) {
            if let Err(err) = functionality.send_message(MSG_INSTALL_STATUS, &payload) {
                error!(error = ?err, "Failed to send installer status");
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

    async fn handle_update_check(&mut self, functionality: &BackendReplier<Self>) {
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

    async fn handle_update_download(&mut self, functionality: &BackendReplier<Self>) {
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

        match self.updater.download_and_apply_update(&download_url).await {
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

    async fn send_update_status(&self, functionality: &BackendReplier<Self>) {
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

    async fn begin_restart_countdown(&mut self, functionality: &BackendReplier<Self>) {
        self.update_in_progress = false;
        self.update_error = None;
        self.pending_update_url = None;
        self.update_pending_restart = true;
        self.update_restart_seconds_remaining = Some(UPDATE_RESTART_DELAY_SECS as u32);
        self.update_progress_message = Some("Update installed. Restarting shortly...".to_string());
        self.send_update_status(functionality).await;
        self.schedule_delayed_restart();
    }

    fn schedule_delayed_restart(&self) {
        tokio::spawn(async {
            sleep(Duration::from_secs(UPDATE_RESTART_DELAY_SECS)).await;
            warn!("Restarting backend after update countdown finished...");
            std::process::exit(0);
        });
    }

    async fn handle_update_restart_request(&mut self, functionality: &BackendReplier<Self>) {
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

async fn drive_syncthing_stream(functionality: BackendReplier<SyncthingBackend>, config: Config) {
    let mut client: Option<SyncthingClient> = None;
    let mut last_event_id: u64 = 0;
    let mut last_emit = Instant::now();

    loop {
        if client.is_none() {
            match SyncthingClient::discover(&config).await {
                Ok(new_client) => {
                    client = Some(new_client);
                    last_event_id = 0;
                    last_emit = Instant::now() - Duration::from_secs(EVENT_HEARTBEAT_SECS);
                }
                Err(err) => {
                    warn!(error = ?err, "Failed to initialize Syncthing event watcher");
                    sleep(Duration::from_secs(EVENT_RECONNECT_DELAY_SECS)).await;
                    continue;
                }
            }
        }

        let timeout = Duration::from_secs(EVENT_STREAM_TIMEOUT_SECS);
        let wait_result = client
            .as_mut()
            .expect("client present")
            .wait_for_updates(last_event_id, timeout)
            .await;

        match wait_result {
            Ok(result) => {
                last_event_id = result.last_event_id;
                let due_to_event = result.has_updates;
                let heartbeat_due = last_emit.elapsed().as_secs() >= EVENT_HEARTBEAT_SECS;
                if due_to_event || heartbeat_due {
                    let reason = if due_to_event {
                        "syncthing-event"
                    } else {
                        "syncthing-heartbeat"
                    };
                    let mut backend = functionality.backend.lock().await;
                    backend.send_status(&functionality, reason).await;
                    last_emit = Instant::now();
                }
            }
            Err(err) => {
                warn!(error = ?err, "Syncthing event watcher error");
                client = None;
                sleep(Duration::from_secs(EVENT_RECONNECT_DELAY_SECS)).await;
            }
        }
    }
}

async fn drive_systemd_monitor(
    functionality: BackendReplier<SyncthingBackend>,
    config: Config,
) {
    let mut ticker = tokio::time::interval(Duration::from_secs(SYSTEMD_MONITOR_INTERVAL_SECS));
    let mut last_status: Option<SystemdStatus> = None;

    loop {
        ticker.tick().await;
        let status = query_systemd_status(&config).await;
        let changed = match &last_status {
            None => true,
            Some(previous) => systemd_state_changed(previous, &status),
        };

        if changed {
            let mut backend = functionality.backend.lock().await;
            backend
                .send_status(&functionality, "systemd-monitor")
                .await;
        }

        last_status = Some(status);
    }
}

fn systemd_state_changed(previous: &SystemdStatus, current: &SystemdStatus) -> bool {
    previous.active_state != current.active_state
        || previous.sub_state != current.sub_state
        || previous.result != current.result
        || previous.unit_file_state != current.unit_file_state
        || previous.pid != current.pid
}
