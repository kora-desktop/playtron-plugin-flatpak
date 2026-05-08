use log::{info, warn};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Filters;
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

fn is_game_category(categories: &[String]) -> bool {
    categories.iter().any(|c| c.contains("Game"))
}

const CONTROLLER_TERMS: &[&str] = &["gamepad", "controller", "joystick", "joypad"];

fn has_controller_keywords(keywords: &Option<Vec<String>>, description: &str) -> bool {
    let desc_lower = description.to_lowercase();
    let empty = Vec::new();
    let kw = keywords.as_ref().unwrap_or(&empty);
    CONTROLLER_TERMS.iter().any(|term| {
        kw.iter().any(|k| k.to_lowercase().contains(term))
            || desc_lower.contains(term)
    })
}

struct AppXmlData {
    touchscreen_support: bool,
    touchscreen_recommended: bool,
    is_foss: bool,
}

fn parse_appstream_xml_data(xml_bytes: &[u8]) -> HashMap<String, AppXmlData> {
    let mut result = HashMap::new();
    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_id: Option<String> = None;
    let mut in_component = false;
    let mut in_id = false;
    let mut in_supports = false;
    let mut in_recommends = false;
    let mut in_control = false;
    let mut in_project_license = false;
    let mut has_touch_supports = false;
    let mut has_touch_recommends = false;
    let mut project_license: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"component" => {
                    in_component = true;
                    current_id = None;
                    has_touch_supports = false;
                    has_touch_recommends = false;
                    project_license = None;
                }
                b"id" if in_component && !in_supports && !in_recommends => {
                    in_id = true;
                }
                b"project_license" if in_component => {
                    in_project_license = true;
                }
                b"supports" if in_component => {
                    in_supports = true;
                }
                b"recommends" if in_component => {
                    in_recommends = true;
                }
                b"control" if in_supports || in_recommends => {
                    in_control = true;
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"component" => {
                    if let Some(id) = current_id.take() {
                        let is_foss = project_license
                            .as_ref()
                            .map(|l| !l.is_empty() && !l.contains("LicenseRef-proprietary"))
                            .unwrap_or(false);
                        result.insert(
                            id,
                            AppXmlData {
                                touchscreen_support: has_touch_supports || has_touch_recommends,
                                touchscreen_recommended: has_touch_recommends,
                                is_foss,
                            },
                        );
                    }
                    in_component = false;
                }
                b"id" => in_id = false,
                b"project_license" => in_project_license = false,
                b"supports" => in_supports = false,
                b"recommends" => in_recommends = false,
                b"control" => in_control = false,
                _ => {}
            },
            Ok(Event::Text(ref e)) => {
                if in_id && current_id.is_none() {
                    if let Ok(text) = e.unescape() {
                        current_id = Some(text.to_string());
                    }
                }
                if in_project_license && project_license.is_none() {
                    if let Ok(text) = e.unescape() {
                        project_license = Some(text.to_string());
                    }
                }
                if in_control {
                    if let Ok(text) = e.unescape() {
                        if text.trim() == "touch" {
                            if in_supports {
                                has_touch_supports = true;
                            }
                            if in_recommends {
                                has_touch_recommends = true;
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                warn!("Error parsing AppStream XML: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    result
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CatalogCache {
    pub items: Vec<CatalogEntry>,
    pub fetched_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CatalogEntry {
    pub app_id: String,
    pub name: String,
    pub verified: bool,
    pub is_game: bool,
    pub has_controller_support: bool,
    #[serde(default)]
    pub touchscreen_support: bool,
    #[serde(default)]
    pub touchscreen_recommended: bool,
    #[serde(default)]
    pub is_foss: bool,
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
        let items = Self::fetch_all_apps(&client).await?;
        let fetched_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        info!("Fetched {} total apps from Flathub", items.len());

        Ok(Self { items, fetched_at })
    }

    pub fn to_provider_items(&self, filters: &Filters) -> Vec<ProviderItem> {
        self.items
            .iter()
            .filter(|entry| {
                if !filters.app_allowlist.is_empty() {
                    return filters.app_allowlist.contains(&entry.app_id);
                }
                (!filters.verified_only || entry.verified)
                    && (!filters.games_only || entry.is_game)
                    && (!filters.controller_support_mandatory || entry.has_controller_support)
                    && (!filters.touchscreen_support_mandatory || entry.touchscreen_recommended)
                    && (!filters.foss_only || entry.is_foss)
            })
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

    async fn fetch_all_apps(client: &FlathubClient) -> ResultWithError<Vec<CatalogEntry>> {
        let xml_data = match client.download_appstream_xml().await {
            Ok(xml_bytes) => {
                let map = parse_appstream_xml_data(&xml_bytes);
                info!("Parsed AppStream XML data for {} apps", map.len());
                map
            }
            Err(e) => {
                warn!("Failed to download AppStream XML: {}", e);
                HashMap::new()
            }
        };

        let collection = client.get_verified_collection(1, 1000).await?;
        let mut entries: Vec<CatalogEntry> = collection
            .hits
            .into_iter()
            .map(|hit| {
                let is_game = hit
                    .main_categories
                    .as_str()
                    .map(|c| c.eq_ignore_ascii_case("game"))
                    .unwrap_or(false);
                let xml = xml_data.get(&hit.app_id);
                CatalogEntry {
                    app_id: hit.app_id,
                    name: hit.name.unwrap_or_default(),
                    verified: true,
                    is_game,
                    has_controller_support: false,
                    touchscreen_support: xml.map_or(false, |x| x.touchscreen_support),
                    touchscreen_recommended: xml.map_or(false, |x| x.touchscreen_recommended),
                    is_foss: xml.map_or(false, |x| x.is_foss),
                }
            })
            .collect();
        info!("Fetched {} verified apps from collection", entries.len());

        // Enrich collection entries with controller support from appstream
        let semaphore = Arc::new(Semaphore::new(50));
        let mut controller_handles = Vec::new();

        for entry in &entries {
            let client = client.clone();
            let sem = semaphore.clone();
            let app_id = entry.app_id.clone();
            controller_handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let appstream = client.get_appstream(&app_id).await.ok()?;
                let has_support = has_controller_keywords(
                    &appstream.keywords,
                    appstream.description.as_deref().unwrap_or(""),
                );
                Some((app_id, has_support))
            }));
        }

        let mut controller_map: HashMap<String, bool> = HashMap::new();
        for handle in controller_handles {
            if let Ok(Some((app_id, has_support))) = handle.await {
                controller_map.insert(app_id, has_support);
            }
        }

        for entry in &mut entries {
            if let Some(&has_support) = controller_map.get(&entry.app_id) {
                entry.has_controller_support = has_support;
            }
        }

        let known_ids: HashSet<String> = entries.iter().map(|e| e.app_id.clone()).collect();

        let all_ids = client.get_all_app_ids().await?;
        let remaining: Vec<String> = all_ids
            .into_iter()
            .filter(|id| !known_ids.contains(id))
            .collect();
        info!(
            "Fetching name and verification status for {} additional apps",
            remaining.len()
        );

        let semaphore = Arc::new(Semaphore::new(50));
        let mut handles = Vec::new();

        for app_id in remaining {
            let client = client.clone();
            let sem = semaphore.clone();
            let xml = xml_data.get(&app_id).map(|x| (x.touchscreen_support, x.touchscreen_recommended, x.is_foss));
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let verified = client
                    .get_verification_status(&app_id)
                    .await
                    .ok()
                    .map(|s| s.verified)
                    .unwrap_or(false);
                let appstream = client.get_appstream(&app_id).await.ok();
                let name = appstream
                    .as_ref()
                    .and_then(|info| info.name.clone())
                    .unwrap_or_default();
                let is_game = appstream
                    .as_ref()
                    .map(|info| is_game_category(&info.categories))
                    .unwrap_or(false);
                let has_controller_support = appstream
                    .as_ref()
                    .map(|info| {
                        has_controller_keywords(
                            &info.keywords,
                            info.description.as_deref().unwrap_or(""),
                        )
                    })
                    .unwrap_or(false);
                Some(CatalogEntry {
                    app_id,
                    name,
                    verified,
                    is_game,
                    has_controller_support,
                    touchscreen_support: xml.map_or(false, |x| x.0),
                    touchscreen_recommended: xml.map_or(false, |x| x.1),
                    is_foss: xml.map_or(false, |x| x.2),
                })
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
