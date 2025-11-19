//! Shared helpers for tracking and rendering deployment download progress.

use crate::deployment::DownloadProgress;
use crate::utils::format_bytes;

const DOWNLOAD_PROGRESS_BYTE_STEP: u64 = 512 * 1024;

pub fn render_download_progress_message(prefix: &str, progress: &DownloadProgress) -> String {
    match progress.total_bytes {
        Some(total) => {
            let percent = progress.percent().unwrap_or(0);
            format!(
                "{} ({} / {} - {}%)...",
                prefix,
                format_bytes(progress.downloaded_bytes),
                format_bytes(total),
                percent
            )
        }
        None => format!(
            "{} ({} downloaded)...",
            prefix,
            format_bytes(progress.downloaded_bytes)
        ),
    }
}

pub fn should_emit_download_progress(
    progress: &DownloadProgress,
    last_percent: &mut Option<u8>,
    last_bytes: &mut u64,
) -> bool {
    if let Some(percent) = progress.percent() {
        if last_percent.map(|prev| percent > prev).unwrap_or(true) {
            *last_percent = Some(percent);
            true
        } else {
            false
        }
    } else if progress
        .downloaded_bytes
        .saturating_sub(*last_bytes)
        >= DOWNLOAD_PROGRESS_BYTE_STEP
    {
        *last_bytes = progress.downloaded_bytes;
        true
    } else {
        false
    }
}

