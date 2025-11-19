use appload_client::BackendReplier;
use serde_json::json;
use tracing::error;

use crate::systemd::control_service;

use super::super::protocol::{ControlRequest, MSG_CONTROL_RESULT};
use super::super::Backend;

impl Backend {
    pub async fn handle_systemd_control(
        &mut self,
        functionality: &BackendReplier<Self>,
        req: ControlRequest,
    ) {
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
}

