use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::MonitorError;

/// Handles low-level HTTP communication with the Syncthing API.
#[derive(Clone)]
pub struct HttpClient {
    pub(super) api_key: String,
    pub(super) http: Client,
    pub(super) base_urls: Vec<String>,
    pub(super) current_idx: usize,
}

impl HttpClient {
    /// Performs a GET request and deserializes the JSON response.
    pub async fn get_json<T>(&mut self, path: &str) -> Result<T, MonitorError>
    where
        T: DeserializeOwned,
    {
        self.get_json_with_query(path, &()).await
    }

    /// Performs a GET request with query parameters and deserializes the JSON response.
    pub async fn get_json_with_query<T, Q>(
        &mut self,
        path: &str,
        query: &Q,
    ) -> Result<T, MonitorError>
    where
        T: DeserializeOwned,
        Q: Serialize + ?Sized,
    {
        let base = &self.base_urls[self.current_idx.min(self.base_urls.len().saturating_sub(1))];
        let url = format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .query(query)
            .send()
            .await
            .map_err(MonitorError::Http)?;

        if !response.status().is_success() {
            return Err(MonitorError::Syncthing(format!(
                "{} returned {}",
                path,
                response.status()
            )));
        }

        response.json::<T>().await.map_err(MonitorError::Http)
    }

    /// Performs a PUT request with a JSON body.
    pub async fn put_json<T>(&mut self, path: &str, body: &T) -> Result<(), MonitorError>
    where
        T: Serialize,
    {
        let base = &self.base_urls[self.current_idx.min(self.base_urls.len().saturating_sub(1))];
        let url = format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/'));

        let response = self
            .http
            .put(url)
            .header("X-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(MonitorError::Http)?;

        if !response.status().is_success() {
            return Err(MonitorError::Syncthing(format!(
                "{} returned {}",
                path,
                response.status()
            )));
        }

        Ok(())
    }

    /// Performs a POST request with an empty body.
    pub async fn post(&mut self, path: &str) -> Result<(), MonitorError> {
        let base = &self.base_urls[self.current_idx.min(self.base_urls.len().saturating_sub(1))];
        let url = format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/'));

        let response = self
            .http
            .post(url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(MonitorError::Http)?;

        if !response.status().is_success() {
            return Err(MonitorError::Syncthing(format!(
                "{} returned {}",
                path,
                response.status()
            )));
        }

        Ok(())
    }

    /// Creates a new HttpClient with the given configuration.
    pub fn new(api_key: String, http: Client, base_urls: Vec<String>) -> Self {
        Self {
            api_key,
            http,
            base_urls,
            current_idx: 0,
        }
    }
}

