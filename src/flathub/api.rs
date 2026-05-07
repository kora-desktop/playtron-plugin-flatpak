use log::debug;
use reqwest::Client;
use std::time::Duration;

use crate::constants;
use crate::types::results::ResultWithError;

use super::types::{AppstreamInfo, CollectionResponse, SearchHit, SearchRequest, SearchResponse, VerificationStatus};

#[derive(Clone)]
pub struct FlathubClient {
    client: Client,
    base_url: String,
}

impl Default for FlathubClient {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl FlathubClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to create HTTP client");

        Self {
            client,
            base_url: constants::FLATHUB_API_BASE.to_string(),
        }
    }

    pub async fn search(&self, query: &str) -> ResultWithError<Vec<SearchHit>> {
        let url = format!("{}/search", self.base_url);
        debug!("Searching Flathub: {}", query);

        let request = SearchRequest {
            query: query.to_string(),
        };

        let response = self.client.post(&url).json(&request).send().await?;
        let search_response: SearchResponse = response.json().await?;
        Ok(search_response.hits)
    }

    pub async fn get_appstream(&self, app_id: &str) -> ResultWithError<AppstreamInfo> {
        let url = format!("{}/appstream/{}", self.base_url, app_id);
        debug!("Fetching appstream for: {}", app_id);

        let response = self.client.get(&url).send().await?;
        let info: AppstreamInfo = response.json().await?;
        Ok(info)
    }

    pub async fn get_verification_status(
        &self,
        app_id: &str,
    ) -> ResultWithError<VerificationStatus> {
        let url = format!("{}/verification/{}/status", self.base_url, app_id);
        debug!("Fetching verification status for: {}", app_id);

        let response = self.client.get(&url).send().await?;
        let status: VerificationStatus = response.json().await?;
        Ok(status)
    }

    pub async fn get_all_app_ids(&self) -> ResultWithError<Vec<String>> {
        let url = format!("{}/appstream", self.base_url);
        debug!("Fetching all app IDs");

        let response = self.client.get(&url).send().await?;
        let ids: Vec<String> = response.json().await?;
        Ok(ids)
    }

    pub async fn get_verified_collection(&self, page: u32, per_page: u32) -> ResultWithError<CollectionResponse> {
        let url = format!("{}/collection/verified", self.base_url);
        debug!("Fetching verified collection page={} per_page={}", page, per_page);

        let response = self.client
            .get(&url)
            .query(&[("page", page.to_string()), ("per_page", per_page.to_string())])
            .send()
            .await?;
        let collection: CollectionResponse = response.json().await?;
        Ok(collection)
    }
}
