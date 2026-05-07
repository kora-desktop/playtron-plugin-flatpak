use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::{CATALOG_CACHE_FILENAME, LIBRARY_PROVIDER_ID, PLUGIN_DATA_SUBDIR};
use crate::flathub::api::FlathubClient;
use crate::types::app::{AppType, ProviderItem, ReleaseState};
use crate::types::results::ResultWithError;

const CACHE_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60; // 1 week

#[derive(Serialize, Deserialize)]
pub struct CatalogCache {
    pub items: Vec<CatalogEntry>,
    pub fetched_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CatalogEntry {
    pub app_id: String,
    pub name: String,
}

impl CatalogCache {
    pub async fn load_or_fetch() -> ResultWithError<Self> {
        if let Some(cached) = Self::load_from_disk() {
            if !cached.is_stale() {
                info!(
                    "Loaded catalog from cache ({} items)",
                    cached.items.len()
                );
                return Ok(cached);
            }
            info!("Catalog cache is stale, refreshing");
        }

        let cache = Self::fetch_fresh().await?;
        cache.save_to_disk();
        Ok(cache)
    }

    pub async fn fetch_fresh() -> ResultWithError<Self> {
        let client = FlathubClient::new();
        let items = Self::fetch_all_verified(&client).await?;
        let fetched_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        info!("Fetched {} verified apps from Flathub", items.len());

        Ok(Self { items, fetched_at })
    }

    pub fn to_provider_items(&self) -> Vec<ProviderItem> {
        self.items
            .iter()
            .map(|entry| ProviderItem {
                id: entry.app_id.clone(),
                name: entry.name.clone(),
                provider: LIBRARY_PROVIDER_ID.to_string(),
                app_type: AppType::Game,
                release_state: ReleaseState::Released,
                release_date: 0,
            })
            .collect()
    }

    fn is_stale(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now - self.fetched_at > CACHE_MAX_AGE_SECS
    }

    fn cache_path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join(PLUGIN_DATA_SUBDIR).join(CATALOG_CACHE_FILENAME))
    }

    async fn fetch_all_verified(client: &FlathubClient) -> ResultWithError<Vec<CatalogEntry>> {
        const PER_PAGE: u32 = 250;
        const MAX_PAGES: u32 = 100;

        let mut all_entries = Vec::new();
        let mut page: u32 = 1;

        loop {
            let response = client.get_verified_collection(page, PER_PAGE).await?;

            let is_empty = response.hits.is_empty();
            all_entries.extend(response.hits.into_iter().map(|hit| CatalogEntry {
                app_id: hit.app_id,
                name: hit.name.unwrap_or_default(),
            }));

            let total_pages = response.total_pages.unwrap_or(1);
            if page >= total_pages || is_empty || page >= MAX_PAGES {
                break;
            }

            page += 1;
        }

        Ok(all_entries)
    }

    fn save_to_disk(&self) {
        let Some(path) = Self::cache_path() else {
            return;
        };

        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                warn!("Failed to create cache directory: {}", e);
                return;
            }
        }

        match serde_json::to_string(self) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    warn!("Failed to write catalog cache: {}", e);
                }
            }
            Err(e) => warn!("Failed to serialize catalog cache: {}", e),
        }
    }

    fn load_from_disk() -> Option<Self> {
        let path = Self::cache_path()?;
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }
}
