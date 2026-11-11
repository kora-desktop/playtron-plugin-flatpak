#![allow(clippy::too_many_arguments)]
use crate::constants::{LIBRARY_PROVIDER_ID, LIBRARY_PROVIDER_NAME};
use crate::flatpak::service::FlatpakService;
use crate::types::app::{DownloadStage, EulaEntry, InstalledApp, LaunchOption, ProviderItem};
use crate::types::cloud_sync::CloudPath;
use std::collections::HashMap;
use tokio::sync::mpsc;
use zbus::fdo;
use zbus::object_server::SignalEmitter;
use zbus_macros::interface;

pub struct LibraryProvider {
    pub service: FlatpakService,
}

impl LibraryProvider {
    pub fn new(service: FlatpakService) -> Self {
        LibraryProvider { service }
    }
}

#[interface(name = "inc.kora.plugin.LibraryProvider")]
impl LibraryProvider {
    #[zbus(property)]
    async fn name(&self) -> &str {
        LIBRARY_PROVIDER_NAME
    }

    #[zbus(property)]
    async fn provider(&self) -> &str {
        LIBRARY_PROVIDER_ID
    }

    #[zbus(property)]
    async fn install_target(&self) -> &str {
        "flatpak"
    }

    #[zbus(signal)]
    pub async fn app_new_version_found(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        version: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn install_progressed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        stage: DownloadStage,
        downloaded_bytes: u64,
        total_download_size: u64,
        progress: f64,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn install_started(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        version: String,
        install_directory: String,
        total_download_size: u64,
        requires_internet_connection: bool,
        os: String,
        language: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn install_completed(emitter: &SignalEmitter<'_>, value: String) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn install_failed(
        emitter: &SignalEmitter<'_>,
        app_id: &str,
        error: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn installed_apps_updated(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn launch_error(
        emitter: &SignalEmitter<'_>,
        app_id: &str,
        error: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn launch_ready(emitter: &SignalEmitter<'_>, app_id: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn library_updated(
        emitter: &SignalEmitter<'_>,
        new_items: &[ProviderItem],
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn move_item_completed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        install_folder: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn move_item_failed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        error: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn move_item_progressed(
        emitter: &SignalEmitter<'_>,
        app_id: String,
        progress: f32,
    ) -> zbus::Result<()>;

    async fn get_eulas(
        &self,
        _app_id: &str,
        _country: &str,
        _locale: &str,
    ) -> fdo::Result<Vec<EulaEntry>> {
        Ok(Vec::new())
    }

    async fn get_install_options(
        &self,
        _app_id: &str,
    ) -> fdo::Result<Vec<(String, String, Vec<String>)>> {
        Ok(Vec::new())
    }

    async fn get_installed_apps(&self) -> fdo::Result<Vec<InstalledApp>> {
        log::info!("get_installed_apps");
        match self.service.get_installed_apps().await {
            Ok(apps) => Ok(apps),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn get_provider_item(&self, app_id: String) -> fdo::Result<ProviderItem> {
        match self.service.get_provider_item(&app_id).await {
            Ok(item) => Ok(item),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn get_provider_items(&self) -> fdo::Result<Vec<ProviderItem>> {
        log::info!("get_provider_items");
        match self.service.get_provider_items().await {
            Ok(items) => Ok(items),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn get_post_install_steps(&self, _app_id: &str) -> fdo::Result<String> {
        Ok("[]".to_string())
    }

    async fn get_launch_options(
        &self,
        app_id: &str,
        _params: HashMap<String, zbus::zvariant::Value<'_>>,
    ) -> fdo::Result<Vec<LaunchOption>> {
        match self.service.get_launch_options(app_id).await {
            Ok(options) => Ok(options),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn get_item_metadata(&self, app_id: &str) -> fdo::Result<String> {
        match self.service.get_item_metadata(app_id).await {
            Ok(metadata) => Ok(metadata),
            Err(err) => Err(fdo::Error::Failed(err.to_string())),
        }
    }

    async fn install(
        &self,
        app_id: &str,
        _dest_path: &str,
        _options: HashMap<String, zbus::zvariant::Value<'_>>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<i32> {
        let app_id_owned = app_id.to_string();

        let (tx, mut rx) = mpsc::channel(32);

        let service = self.service.clone();
        let app_id_task = app_id_owned.clone();
        let install_handle =
            tokio::spawn(async move { service.install_with_progress(&app_id_task, tx).await });

        let install_dir = format!(
            "{}/.local/share/flatpak/app/{}",
            dirs::home_dir().unwrap_or_default().to_string_lossy(),
            app_id
        );

        let mut started = false;
        while let Some(progress) = rx.recv().await {
            if !started {
                emitter
                    .install_started(
                        app_id_owned.clone(),
                        String::new(),
                        install_dir.clone(),
                        progress.total_size,
                        true,
                        "linux".to_string(),
                        String::new(),
                    )
                    .await
                    .ok();
                started = true;
            }
            emitter
                .install_progressed(
                    app_id_owned.clone(),
                    progress.stage,
                    progress.bytes_transferred,
                    progress.total_size,
                    progress.progress,
                )
                .await
                .ok();
        }

        match install_handle.await {
            Ok(Ok(())) => {
                emitter.install_completed(app_id_owned).await.ok();
                Ok(0)
            }
            Ok(Err(e)) => {
                emitter
                    .install_failed(app_id, &e.to_string())
                    .await
                    .ok();
                Err(fdo::Error::Failed(e.to_string()))
            }
            Err(e) => {
                emitter
                    .install_failed(app_id, &e.to_string())
                    .await
                    .ok();
                Err(fdo::Error::Failed(e.to_string()))
            }
        }
    }

    async fn move_item(
        &self,
        _app_id: String,
        _dest_path: String,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "Move is not supported for Flatpak apps".to_string(),
        ))
    }

    async fn cancel_move_item(&self, _app_id: &str) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "Move is not supported for Flatpak apps".to_string(),
        ))
    }

    async fn uninstall(&self, app_id: &str) -> fdo::Result<()> {
        self.service.uninstall(app_id).await
    }

    async fn update(
        &self,
        app_id: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        let (tx, mut rx) = mpsc::channel(32);

        let service = self.service.clone();
        let app_id_task = app_id.clone();
        let update_handle =
            tokio::spawn(async move { service.update_with_progress(&app_id_task, tx).await });

        while let Some(progress) = rx.recv().await {
            emitter
                .install_progressed(
                    app_id.clone(),
                    progress.stage,
                    progress.bytes_transferred,
                    0,
                    progress.progress,
                )
                .await
                .ok();
        }

        match update_handle.await {
            Ok(Ok(())) => {
                emitter.install_completed(app_id).await.ok();
                Ok(())
            }
            Ok(Err(e)) => Err(fdo::Error::Failed(e.to_string())),
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    async fn get_save_path_patterns(
        &self,
        app_id: &str,
        platform: &str,
    ) -> fdo::Result<Vec<CloudPath>> {
        self.service.get_save_path_patterns(app_id, platform).await
    }

    fn pause_install(&self) {
        log::info!("pause_install");
    }

    async fn refresh(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        log::info!("refresh");
        match self.service.refresh().await {
            Ok(items) => {
                emitter.library_updated(&items).await.ok();
                Ok(())
            }
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    async fn pre_launch_hook(
        &self,
        app_id: &str,
        _using_offline_mode: bool,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<Vec<String>> {
        emitter.launch_ready(app_id).await?;
        self.service.pre_launch_hook(app_id).await
    }

    async fn post_launch_hook(&self, app_id: &str) -> fdo::Result<()> {
        log::info!("post_launch_hook for {}", app_id);
        Ok(())
    }

    async fn sync_installed_apps(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> fdo::Result<()> {
        log::info!("sync_installed_apps");
        match self.service.sync_installed_apps().await {
            Ok(items) => {
                emitter.library_updated(&items).await.ok();
                Ok(())
            }
            Err(e) => Err(fdo::Error::Failed(e.to_string())),
        }
    }

    async fn import(&self, app_id: &str, install_folder: &str) -> fdo::Result<()> {
        log::info!("import {} from {}", app_id, install_folder);
        self.service
            .import(app_id, install_folder)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))
    }
}
