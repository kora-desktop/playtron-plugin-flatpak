use std::env;
use std::future::pending;
mod auth;
mod config;
mod constants;
mod flatpak;
mod flathub;
mod plugin;
mod types;
mod utils;

use crate::flatpak::client::FlatpakClient;
use crate::flatpak::service::FlatpakService;
use crate::plugin::dbus::{build_connection, register_plugin};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let version = env!("CARGO_PKG_VERSION");

    env::set_var("RUST_LOG", "debug");
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .init();

    log::info!("Starting Playtron Flatpak Plugin version: {version}");

    FlatpakClient::ensure_flathub_remote().await?;

    let service = FlatpakService::new().await;
    build_connection(service).await?;
    register_plugin().await;

    tokio::spawn(flatpak::watcher::start_watcher());

    pending::<()>().await;

    Ok(())
}
