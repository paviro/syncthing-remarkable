mod app;
mod config;
mod deployment;
mod syncthing_client;
mod systemd;
mod types;
mod utils;

use appload_client::AppLoad;
use tracing::error;
use tracing_subscriber::EnvFilter;

use crate::app::Backend;
use crate::config::Config;

#[tokio::main]
async fn main() {
    init_tracing();
    let config = Config::load().await;
    let monitor = Backend::new(config).await;
    match AppLoad::new(monitor) {
        Ok(mut app) => {
            if let Err(err) = app.run().await {
                error!(error = ?err, "AppLoad backend exited with error");
            }
        }
        Err(err) => error!(error = ?err, "Failed to start AppLoad backend"),
    }
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();
}

