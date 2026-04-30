use crate::constants::{BUS_NAME, CLIENT_PATH};
use crate::flatpak::service::FlatpakService;
use crate::types::results::EmptyResult;
use crate::{auth, plugin};
use tokio::sync::Mutex;
use zbus::{connection, zvariant::ObjectPath, Connection};

lazy_static::lazy_static! {
    pub static ref CONNECTION: Mutex<Option<Connection>> =
        Mutex::new(None);
}

pub async fn build_connection(service: FlatpakService) -> EmptyResult {
    let plugin = plugin::plugin_interface::Plugin {};
    let cryptography = auth::cryptography::Cryptography::new(service.clone());
    let library_provider = plugin::library_provider::LibraryProvider::new(service.clone());

    *CONNECTION.lock().await = Some(
        connection::Builder::session()?
            .name(BUS_NAME)?
            .serve_at(CLIENT_PATH, plugin)?
            .serve_at(CLIENT_PATH, cryptography)?
            .serve_at(CLIENT_PATH, library_provider)?
            .build()
            .await?,
    );
    Ok(())
}

pub async fn register_plugin() {
    if let Some(conn) = CONNECTION.lock().await.as_ref() {
        let object_path = match ObjectPath::try_from(CLIENT_PATH) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Invalid object path: {}", e);
                return;
            }
        };

        for attempt in 1..=5 {
            match conn
                .call_method(
                    Some("one.playtron.Playserve"),
                    "/one/playtron/plugins/Manager",
                    Some("one.playtron.plugin.Manager"),
                    "RegisterPlugin",
                    &(BUS_NAME, &object_path),
                )
                .await
            {
                Ok(_) => {
                    log::info!("The plugin registered successfully");
                    return;
                }
                Err(error) => {
                    if attempt < 5 {
                        log::warn!(
                            "Registration attempt {}/5 failed: {}. Retrying...",
                            attempt, error
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(attempt)).await;
                    } else {
                        log::error!(
                            "The plugin failed to register after 5 attempts: {}",
                            error
                        );
                    }
                }
            }
        }
    }
}
