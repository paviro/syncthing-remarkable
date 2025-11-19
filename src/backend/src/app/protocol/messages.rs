/// Message type constants for communication between frontend and backend

// Request messages (from frontend)
pub const MSG_SYSTEM_NEW_COORDINATOR: u32 = appload_client::MSG_SYSTEM_NEW_COORDINATOR;
pub const MSG_CONTROL_REQUEST: u32 = 1;
pub const MSG_INSTALL_TRIGGER: u32 = 2;
pub const MSG_GUI_ADDRESS_TOGGLE: u32 = 3;
pub const MSG_UPDATE_CHECK_REQUEST: u32 = 4;
pub const MSG_UPDATE_DOWNLOAD_REQUEST: u32 = 5;
pub const MSG_UPDATE_RESTART_REQUEST: u32 = 6;

// Response messages (to frontend)
pub const MSG_STATUS_UPDATE: u32 = 100;
pub const MSG_CONTROL_RESULT: u32 = 101;
pub const MSG_INSTALL_STATUS: u32 = 102;
pub const MSG_GUI_ADDRESS_RESULT: u32 = 103;
pub const MSG_UPDATE_CHECK_RESULT: u32 = 104;
pub const MSG_UPDATE_DOWNLOAD_STATUS: u32 = 105;
pub const MSG_ERROR: u32 = 500;

// Timing constants
pub const UPDATE_RESTART_DELAY_SECS: u64 = 10;
pub const EVENT_STREAM_TIMEOUT_SECS: u64 = 30;
pub const EVENT_HEARTBEAT_SECS: u64 = 5;
pub const EVENT_RECONNECT_DELAY_SECS: u64 = 5;
pub const SYSTEMD_MONITOR_INTERVAL_SECS: u64 = 5;

