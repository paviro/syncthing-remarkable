use serde::Deserialize;

use crate::systemd::ServiceAction;

#[derive(Debug, Deserialize)]
pub struct ControlRequest {
    pub action: ServiceAction,
}

#[derive(Debug, Deserialize)]
pub struct GuiAddressToggleRequest {
    pub address: String,
}

