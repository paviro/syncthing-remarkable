use serde::Serialize;

#[derive(Debug, Serialize, Default, Clone, PartialEq, Eq)]
pub struct SystemdStatus {
    pub name: String,
    pub active_state: Option<String>,
    pub sub_state: Option<String>,
    pub unit_file_state: Option<String>,
    pub result: Option<String>,
    pub pid: Option<u32>,
    pub active_enter_timestamp: Option<String>,
    pub inactive_enter_timestamp: Option<String>,
    pub description: Option<String>,
    pub raw_excerpt: Option<String>,
    pub error: Option<String>,
}

