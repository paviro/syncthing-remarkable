use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
}

impl ServiceAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceAction::Start => "start",
            ServiceAction::Stop => "stop",
            ServiceAction::Restart => "restart",
            ServiceAction::Enable => "enable",
            ServiceAction::Disable => "disable",
        }
    }

    pub fn past_tense(&self) -> &'static str {
        match self {
            ServiceAction::Start => "started",
            ServiceAction::Stop => "stopped",
            ServiceAction::Restart => "restarted",
            ServiceAction::Enable => "enabled",
            ServiceAction::Disable => "disabled",
        }
    }

    pub fn needs_remount(&self) -> bool {
        matches!(self, ServiceAction::Enable | ServiceAction::Disable)
    }
}

