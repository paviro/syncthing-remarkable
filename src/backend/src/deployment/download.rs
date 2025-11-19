//! Shared download and extraction helpers for deployment workflows.

use reqwest::Client;
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::deployment::{DownloadProgress, DownloadProgressSender};
use crate::types::MonitorError;

const DOWNLOAD_TIMEOUT_SECS: u64 = 10 * 60;

pub async fn download_to_path(
    client: &Client,
    url: &str,
    destination: &Path,
    progress_tx: Option<DownloadProgressSender>,
) -> Result<(), MonitorError> {
    let timeout = Duration::from_secs(DOWNLOAD_TIMEOUT_SECS);
    let request = client.get(url).timeout(timeout);

    let mut response = request.send().await?.error_for_status()?;
    let mut file = File::create(destination).await?;
    let mut downloaded_bytes: u64 = 0;
    let total_bytes = response.content_length();

    emit_progress(progress_tx.as_ref(), downloaded_bytes, total_bytes).await;

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded_bytes = downloaded_bytes.saturating_add(chunk.len() as u64);
        emit_progress(progress_tx.as_ref(), downloaded_bytes, total_bytes).await;
    }

    file.flush().await?;
    Ok(())
}

async fn emit_progress(
    progress_tx: Option<&DownloadProgressSender>,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) {
    if let Some(progress_tx) = progress_tx {
        let _ = progress_tx
            .send(DownloadProgress {
                downloaded_bytes,
                total_bytes,
            })
            .await;
    }
}

