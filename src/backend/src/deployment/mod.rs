//! Deployment workflows and utilities for Syncthing installation and updates.

pub mod http;
pub mod system;
pub mod types;
pub mod util;
pub mod workflows;

// Re-export commonly used items for convenience
pub use types::*;
pub use util::progress::*;
pub use workflows::installer::Installer;
pub use workflows::updater::Updater;
