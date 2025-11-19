mod actions;
mod client;
mod monitor;
mod types;

pub use actions::ServiceAction;
pub use client::{control_service, query_status};
pub use monitor::monitor_service;
pub use types::SystemdStatus;

