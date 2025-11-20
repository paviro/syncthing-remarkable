use appload_client::BackendReplier;
use serde_json::json;
use tracing::{error, info, warn};

use crate::systemd::{control_service, ServiceAction};
use crate::syncthing_client::SyncthingClient;

use super::super::protocol::{
    ControlRequest, GuiAddressToggleRequest, MSG_CONTROL_RESULT, MSG_GUI_ADDRESS_RESULT,
};
use super::super::Backend;

impl Backend {
    /// Handle service control operations (start, stop, restart).
    /// For restart, tries API first then falls back to systemd.
    /// For other operations, uses systemd directly.
    pub async fn handle_service_control(
        &mut self,
        functionality: &BackendReplier<Self>,
        req: ControlRequest,
    ) {
        // For restart actions, try the API first if available
        if matches!(req.action, ServiceAction::Restart) {
            if let Some(result) = self.try_api_restart().await {
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
                return;
            }
        }

        // Fall back to systemd control (or for non-restart actions)
        match control_service(&self.config, req.action).await {
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
        }
    }

    /// Attempts to restart Syncthing via the API.
    /// Returns Some(message) if successful, None if API is unreachable or restart failed.
    async fn try_api_restart(&self) -> Option<String> {
        info!("Attempting to restart Syncthing via API...");

        match SyncthingClient::discover(&self.config).await {
            Ok(mut client) => match client.restart().await {
                Ok(()) => {
                    info!("Successfully restarted Syncthing via API");
                    Some("Syncthing restarted via API".to_string())
                }
                Err(err) => {
                    warn!(error = ?err, "Failed to restart Syncthing via API, will fallback to systemd");
                    None
                }
            },
            Err(err) => {
                warn!(error = ?err, "Failed to connect to Syncthing API for restart, will fallback to systemd");
                None
            }
        }
    }

    /// Handle GUI address changes via Syncthing API
    pub async fn handle_syncthing_gui_listen_address(
        &mut self,
        functionality: &BackendReplier<Self>,
        req: GuiAddressToggleRequest,
    ) {
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
}

