mod api;
mod client;
mod core;
mod helpers;
mod models;

pub use client::SyncthingClient;

// Re-export data types at root for convenience
pub use models::{FolderPayload, PeerPayload, SyncthingOverview};
