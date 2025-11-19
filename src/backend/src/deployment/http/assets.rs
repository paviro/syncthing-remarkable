//! Helpers for selecting release assets for installer and updater flows.

use reqwest::Client;
use serde::Deserialize;

use crate::types::MonitorError;

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

pub async fn fetch_release(client: &Client, url: &str) -> Result<Release, MonitorError> {
    let response = client.get(url).send().await?.error_for_status()?;
    let release: Release = response.json().await?;
    Ok(release)
}

pub fn select_asset_by_prefix<'a>(
    assets: &'a [ReleaseAsset],
    prefix: &str,
    suffix: &str,
) -> Option<&'a ReleaseAsset> {
    assets
        .iter()
        .find(|asset| asset.name.starts_with(prefix) && asset.name.ends_with(suffix))
}

pub fn select_asset_exact<'a>(assets: &'a [ReleaseAsset], name: &str) -> Option<&'a ReleaseAsset> {
    assets.iter().find(|asset| asset.name == name)
}

