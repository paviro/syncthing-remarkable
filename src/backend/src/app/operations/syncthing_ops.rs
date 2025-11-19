use appload_client::BackendReplier;
use serde_json::json;
use tracing::error;

use super::super::protocol::{GuiAddressToggleRequest, MSG_GUI_ADDRESS_RESULT};
use super::super::Backend;

impl Backend {
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

