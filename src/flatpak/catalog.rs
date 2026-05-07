use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::{
    CATALOG_CACHE_FILENAME, LIBRARY_PROVIDER_ID, METADATA_CACHE_FILENAME, PLUGIN_DATA_SUBDIR,
};
use crate::flathub::api::FlathubClient;
use crate::types::app::{
    AppType, ItemMetadata, PlaytronAppType, PlaytronProvider, ProviderItem, ReleaseState,
};
use crate::types::results::ResultWithError;

use super::service::select_best_screenshot;
use tokio::sync::Semaphore;

fn cache_max_age_secs(cache_days: u64) -> u64 {
    cache_days * 24 * 60 * 60
}

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
    pub async fn load_or_fetch(cache_days: u64) -> ResultWithError<Self> {
        if cache_days > 0 {
            if let Some(cached) = Self::load_from_disk() {
                if !cached.is_stale(cache_days) {
                    info!(
                        "Loaded catalog from cache ({} items)",
                        cached.items.len()
                    );
                    return Ok(cached);
                }
                info!("Catalog cache is stale, refreshing");
            }
        }

        let cache = Self::fetch_fresh().await?;
        if cache_days > 0 {
            cache.save_to_disk();
        }
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

    fn is_stale(&self, cache_days: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now - self.fetched_at > cache_max_age_secs(cache_days)
    }

    fn cache_path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join(PLUGIN_DATA_SUBDIR).join(CATALOG_CACHE_FILENAME))
    }

    async fn fetch_all_verified(client: &FlathubClient) -> ResultWithError<Vec<CatalogEntry>> {
        let collection = client.get_verified_collection(1, 1000).await?;
        let mut entries: Vec<CatalogEntry> = collection
            .hits
            .into_iter()
            .map(|hit| CatalogEntry {
                app_id: hit.app_id,
                name: hit.name.unwrap_or_default(),
            })
            .collect();
        info!("Fetched {} verified apps from collection", entries.len());

        let known_ids: HashSet<String> = entries.iter().map(|e| e.app_id.clone()).collect();

        let all_ids = client.get_all_app_ids().await?;
        let remaining: Vec<String> = all_ids
            .into_iter()
            .filter(|id| !known_ids.contains(id))
            .collect();
        info!(
            "Checking {} additional apps for verification",
            remaining.len()
        );

        let semaphore = Arc::new(Semaphore::new(50));
        let mut handles = Vec::new();

        for app_id in remaining {
            let client = client.clone();
            let sem = semaphore.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let status = client.get_verification_status(&app_id).await.ok()?;
                if !status.verified {
                    return None;
                }
                let name = client
                    .get_appstream(&app_id)
                    .await
                    .ok()
                    .and_then(|info| info.name)
                    .unwrap_or_default();
                Some(CatalogEntry { app_id, name })
            }));
        }

        for handle in handles {
            if let Ok(Some(entry)) = handle.await {
                entries.push(entry);
            }
        }

        Ok(entries)
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

#[derive(Serialize, Deserialize)]
pub struct MetadataCache {
    pub entries: HashMap<String, String>,
    pub fetched_at: u64,
}

impl MetadataCache {
    pub async fn load_or_fetch(app_ids: &[String], cache_days: u64) -> HashMap<String, String> {
        if cache_days > 0 {
            if let Some(cached) = Self::load_from_disk() {
                if !cached.is_stale(cache_days) {
                    info!(
                        "Loaded metadata cache from disk ({} entries)",
                        cached.entries.len()
                    );
                    return cached.entries;
                }
                info!("Metadata cache is stale, refreshing");
            }
        }

        match Self::fetch_all(app_ids).await {
            Ok(cache) => {
                if cache_days > 0 {
                    cache.save_to_disk();
                }
                info!("Built metadata cache ({} entries)", cache.entries.len());
                cache.entries
            }
            Err(e) => {
                warn!("Failed to build metadata cache: {}", e);
                HashMap::new()
            }
        }
    }

    async fn fetch_all(app_ids: &[String]) -> ResultWithError<Self> {
        let client = FlathubClient::new();
        let semaphore = Arc::new(Semaphore::new(50));
        let mut handles = Vec::new();

        for app_id in app_ids {
            let client = client.clone();
            let sem = semaphore.clone();
            let app_id = app_id.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let appstream = client.get_appstream(&app_id).await.ok()?;
                let metadata = ItemMetadata {
                    id: app_id.clone(),
                    name: appstream.name.unwrap_or_default(),
                    providers: vec![PlaytronProvider {
                        provider: LIBRARY_PROVIDER_ID.to_string(),
                        provider_app_id: app_id.clone(),
                        store_id: app_id.clone(),
                        namespace: LIBRARY_PROVIDER_ID.to_string(),
                        ..Default::default()
                    }],
                    slug: app_id.clone(),
                    summary: appstream.summary.unwrap_or_default(),
                    description: appstream.description.unwrap_or_default(),
                    tags: Vec::new(),
                    images: select_best_screenshot(&appstream.screenshots),
                    publishers: Vec::new(),
                    developers: appstream
                        .developer_name
                        .map(|d| vec![d])
                        .unwrap_or_default(),
                    app_type: PlaytronAppType::Game,
                    use_container_runtime: false,
                };
                let json = serde_json::to_string(&metadata).ok()?;
                Some((app_id, json))
            }));
        }

        let mut entries = HashMap::new();
        for handle in handles {
            if let Ok(Some((app_id, json))) = handle.await {
                entries.insert(app_id, json);
            }
        }

        let fetched_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(Self { entries, fetched_at })
    }

    fn is_stale(&self, cache_days: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now - self.fetched_at > cache_max_age_secs(cache_days)
    }

    fn cache_path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join(PLUGIN_DATA_SUBDIR).join(METADATA_CACHE_FILENAME))
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
                    warn!("Failed to write metadata cache: {}", e);
                }
            }
            Err(e) => warn!("Failed to serialize metadata cache: {}", e),
        }
    }

    fn load_from_disk() -> Option<Self> {
        let path = Self::cache_path()?;
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }
}
