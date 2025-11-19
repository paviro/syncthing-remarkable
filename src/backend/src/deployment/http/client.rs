//! HTTP client utilities for deployment-related workflows.

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT};
use reqwest::Client;
use std::time::Duration;

use crate::types::MonitorError;

const USER_AGENT: &str = "syncthing-for-remarkable-appload";
pub const GITHUB_ACCEPT_HEADER: &str = "application/vnd.github+json";
pub const GITHUB_API_VERSION_HEADER: &str = "x-github-api-version";
pub const GITHUB_API_VERSION: &str = "2022-11-28";
pub const REQUEST_TIMEOUT_SECS: u64 = 60;

pub fn default_github_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static(GITHUB_ACCEPT_HEADER));
    headers.insert(
        HeaderName::from_static(GITHUB_API_VERSION_HEADER),
        HeaderValue::from_static(GITHUB_API_VERSION),
    );
    headers
}

pub fn github_client(timeout: Duration) -> Result<Client, MonitorError> {
    Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(default_github_headers())
        .timeout(timeout)
        .build()
        .map_err(Into::into)
}

pub fn default_request_timeout() -> Duration {
    Duration::from_secs(REQUEST_TIMEOUT_SECS)
}

