use log::{error, info, warn};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::LineEnding;
use rsa::{RsaPrivateKey, RsaPublicKey};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use zbus::fdo;

use crate::config::{self, Config};
use crate::constants::LIBRARY_PROVIDER_ID;
use crate::flathub::api::FlathubClient;
use crate::flathub::types::SearchHit;
use crate::types::app::{
    AppType, DownloadStage, InstalledApp, ItemMetadata, LaunchOption, LaunchType,
    PlaytronProvider, ProviderItem, ReleaseState,
};
use crate::types::cloud_sync::CloudPath;
use crate::types::results::ResultWithError;

use super::catalog::CatalogCache;
use super::client::FlatpakClient;

pub struct InstallProgress {
    pub stage: DownloadStage,
    pub progress: f64,
    pub bytes_transferred: u64,
}

#[derive(Clone)]
pub struct FlatpakService {
    rsa: RsaPrivateKey,
    config: Config,
    catalog: Vec<ProviderItem>,
}

#[allow(dead_code)]
impl FlatpakService {
    pub async fn new() -> Self {
        let config = config::load_config();
        let mut rng = rand::thread_rng();
        let rsa = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate RSA key");

        let catalog = match CatalogCache::load_or_fetch().await {
            Ok(cache) => cache.to_provider_items(),
            Err(e) => {
                warn!("Failed to load catalog: {}", e);
                Vec::new()
            }
        };

        Self { rsa, config, catalog }
    }

    pub fn get_public_key(&self) -> String {
        let public_key = RsaPublicKey::from(&self.rsa);
        public_key
            .to_pkcs1_pem(LineEnding::LF)
            .map(|pem| pem.to_string())
            .unwrap_or_else(|e| {
                error!("Failed to encode public key: {}", e);
                String::new()
            })
    }

    pub async fn search(&self, query: &str) -> ResultWithError<Vec<SearchHit>> {
        let client = FlathubClient::new();
        let mut hits = client.search(query).await?;

        if self.config.filters.verified_only {
            hits.retain(|hit| hit.verification_verified.unwrap_or(false));
        }

        Ok(hits)
    }

    pub async fn get_installed_apps(&self) -> ResultWithError<Vec<InstalledApp>> {
        let installed = FlatpakClient::list_installed().await?;

        let apps = installed
            .into_iter()
            .map(|flat| {
                let size = parse_size(&flat.size);
                InstalledApp {
                    app_id: flat.app_id.clone(),
                    installed_path: format!(
                        "{}/.local/share/flatpak/app/{}",
                        dirs::home_dir()
                            .unwrap_or_default()
                            .to_string_lossy(),
                        flat.app_id
                    ),
                    downloaded_bytes: size,
                    total_download_size: size,
                    disk_size: size,
                    version: flat.version.clone(),
                    latest_version: flat.version,
                    update_pending: false,
                    os: "linux".to_string(),
                    language: String::new(),
                    disabled_dlc: Vec::new(),
                }
            })
            .collect();

        Ok(apps)
    }

    pub async fn get_provider_items(&self) -> ResultWithError<Vec<ProviderItem>> {
        let installed = FlatpakClient::list_installed().await?;

        let mut items: Vec<ProviderItem> = installed
            .into_iter()
            .map(|flat| ProviderItem {
                id: flat.app_id,
                name: flat.name,
                provider: LIBRARY_PROVIDER_ID.to_string(),
                app_type: AppType::Game,
                release_state: ReleaseState::Released,
                release_date: 0,
            })
            .collect();

        let installed_ids: std::collections::HashSet<String> =
            items.iter().map(|i| i.id.clone()).collect();

        for catalog_item in &self.catalog {
            if !installed_ids.contains(&catalog_item.id) {
                items.push(catalog_item.clone());
            }
        }

        Ok(items)
    }

    pub async fn get_provider_item(&self, app_id: &str) -> ResultWithError<ProviderItem> {
        let installed = FlatpakClient::list_installed().await?;

        if let Some(flat) = installed.into_iter().find(|f| f.app_id == app_id) {
            return Ok(ProviderItem {
                id: flat.app_id,
                name: flat.name,
                provider: LIBRARY_PROVIDER_ID.to_string(),
                app_type: AppType::Game,
                release_state: ReleaseState::Released,
                release_date: 0,
            });
        }

        self.catalog
            .iter()
            .find(|item| item.id == app_id)
            .cloned()
            .ok_or_else(|| format!("App {} not found", app_id).into())
    }

    pub async fn get_item_metadata(&self, app_id: &str) -> ResultWithError<String> {
        let client = FlathubClient::new();
        let appstream = client.get_appstream(app_id).await?;

        let metadata = ItemMetadata {
            id: app_id.to_string(),
            name: appstream.name.unwrap_or_default(),
            providers: vec![PlaytronProvider {
                provider: LIBRARY_PROVIDER_ID.to_string(),
                provider_app_id: app_id.to_string(),
                store_id: app_id.to_string(),
                namespace: LIBRARY_PROVIDER_ID.to_string(),
                ..Default::default()
            }],
            slug: app_id.to_string(),
            summary: appstream.summary.unwrap_or_default(),
            description: appstream.description.unwrap_or_default(),
            tags: Vec::new(),
            images: Vec::new(),
            publishers: Vec::new(),
            developers: appstream
                .developer_name
                .map(|d| vec![d])
                .unwrap_or_default(),
            app_type: crate::types::app::PlaytronAppType::Game,
            use_container_runtime: true,
        };

        Ok(serde_json::to_string(&metadata)?)
    }

    pub async fn get_launch_options(&self, app_id: &str) -> ResultWithError<Vec<LaunchOption>> {
        Ok(vec![LaunchOption {
            description: String::new(),
            executable: "flatpak".to_string(),
            arguments: format!("run --user {}", app_id),
            working_directory: String::new(),
            environment: Vec::new(),
            launch_type: LaunchType::Game,
            hardware_tags: Vec::new(),
        }])
    }

    pub async fn install(&self, app_id: &str) -> ResultWithError<()> {
        let mut child = FlatpakClient::install(app_id).await?;
        let status = child.wait().await?;
        if !status.success() {
            return Err(format!("Failed to install {}", app_id).into());
        }
        Ok(())
    }

    pub async fn install_with_progress(
        &self,
        app_id: &str,
        progress_tx: mpsc::Sender<InstallProgress>,
    ) -> ResultWithError<()> {
        let mut child = FlatpakClient::install(app_id).await?;

        progress_tx
            .send(InstallProgress {
                stage: DownloadStage::Downloading,
                progress: 0.0,
                bytes_transferred: 0,
            })
            .await
            .ok();

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(progress) = parse_progress_line(&line) {
                    progress_tx.send(progress).await.ok();
                }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            return Err(format!("Failed to install {}", app_id).into());
        }

        progress_tx
            .send(InstallProgress {
                stage: DownloadStage::Downloading,
                progress: 100.0,
                bytes_transferred: 0,
            })
            .await
            .ok();

        Ok(())
    }

    pub async fn uninstall(&self, app_id: &str) -> fdo::Result<()> {
        FlatpakClient::uninstall(app_id)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }

    pub async fn update(&self, app_id: &str) -> ResultWithError<()> {
        let mut child = FlatpakClient::update(app_id).await?;
        let status = child.wait().await?;
        if !status.success() {
            return Err(format!("Failed to update {}", app_id).into());
        }
        Ok(())
    }

    pub async fn update_with_progress(
        &self,
        app_id: &str,
        progress_tx: mpsc::Sender<InstallProgress>,
    ) -> ResultWithError<()> {
        let mut child = FlatpakClient::update(app_id).await?;

        progress_tx
            .send(InstallProgress {
                stage: DownloadStage::Downloading,
                progress: 0.0,
                bytes_transferred: 0,
            })
            .await
            .ok();

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(progress) = parse_progress_line(&line) {
                    progress_tx.send(progress).await.ok();
                }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            return Err(format!("Failed to update {}", app_id).into());
        }

        progress_tx
            .send(InstallProgress {
                stage: DownloadStage::Downloading,
                progress: 100.0,
                bytes_transferred: 0,
            })
            .await
            .ok();

        Ok(())
    }

    pub async fn refresh(&self) -> ResultWithError<Vec<ProviderItem>> {
        info!("Refreshing installed flatpak apps");
        self.get_provider_items().await
    }

    pub async fn refresh_catalog(&mut self) -> ResultWithError<()> {
        info!("Refreshing catalog from Flathub");
        let cache = CatalogCache::fetch_fresh().await?;
        self.catalog = cache.to_provider_items();
        Ok(())
    }

    pub async fn pre_launch_hook(&self, _app_id: &str) -> fdo::Result<Vec<String>> {
        Ok(Vec::new())
    }

    pub async fn get_save_path_patterns(
        &self,
        _app_id: &str,
        _platform: &str,
    ) -> fdo::Result<Vec<CloudPath>> {
        Ok(Vec::new())
    }

    pub async fn sync_installed_apps(&self) -> ResultWithError<Vec<ProviderItem>> {
        info!("Syncing installed flatpak apps");
        self.get_provider_items().await
    }

    pub async fn import(&self, app_id: &str, install_folder: &str) -> ResultWithError<()> {
        let path = std::path::Path::new(install_folder);
        let ext = path.extension().and_then(|e| e.to_str());

        match ext {
            Some("flatpakref") => {
                let mut child = FlatpakClient::install_flatpakref(install_folder).await?;
                let status = child.wait().await?;
                if !status.success() {
                    return Err(
                        format!("Failed to import {} from {}", app_id, install_folder).into(),
                    );
                }
                Ok(())
            }
            Some("flatpak") => {
                let mut child = FlatpakClient::install_bundle(install_folder).await?;
                let status = child.wait().await?;
                if !status.success() {
                    return Err(
                        format!("Failed to import {} from {}", app_id, install_folder).into(),
                    );
                }
                Ok(())
            }
            _ => {
                warn!(
                    "Ignoring import for {}: path '{}' is not a flatpak bundle or ref",
                    app_id, install_folder
                );
                Ok(())
            }
        }
    }
}

fn parse_progress_line(line: &str) -> Option<InstallProgress> {
    let lower = line.to_lowercase();

    if lower.contains("downloading") || lower.contains("fetching") {
        Some(InstallProgress {
            stage: DownloadStage::Downloading,
            progress: 25.0,
            bytes_transferred: 0,
        })
    } else if lower.contains("verifying") {
        Some(InstallProgress {
            stage: DownloadStage::Verifying,
            progress: 75.0,
            bytes_transferred: 0,
        })
    } else if lower.contains("installing") || lower.contains("deploying") {
        Some(InstallProgress {
            stage: DownloadStage::Downloading,
            progress: 50.0,
            bytes_transferred: 0,
        })
    } else {
        warn!("Unrecognized flatpak progress line: {}", line);
        None
    }
}

fn parse_size(size_str: &str) -> u64 {
    let s = size_str.trim();
    if s.is_empty() {
        return 0;
    }

    let (num_part, unit) = s
        .find(|c: char| c.is_alphabetic())
        .map(|i| (&s[..i], s[i..].trim()))
        .unwrap_or((s, ""));

    let base: f64 = num_part.trim().replace(',', ".").parse().unwrap_or(0.0);

    let multiplier: u64 = match unit.to_lowercase().as_str() {
        "kb" => 1_000,
        "mb" => 1_000_000,
        "gb" => 1_000_000_000,
        "kib" => 1_024,
        "mib" => 1_048_576,
        "gib" => 1_073_741_824,
        "bytes" | "b" => 1,
        _ => 1,
    };

    (base * multiplier as f64) as u64
}
