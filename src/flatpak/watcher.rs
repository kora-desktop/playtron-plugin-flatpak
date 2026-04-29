use log::{debug, error, info, warn};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use zbus::Connection;

use crate::constants::CLIENT_PATH;
use crate::plugin::library_provider::LibraryProvider;

lazy_static::lazy_static! {
    static ref WATCHER: Mutex<Option<RecommendedWatcher>> = Mutex::new(None);
}

fn flatpak_app_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("flatpak").join("app"))
}

pub async fn start_watcher() {
    let app_dir = match flatpak_app_dir() {
        Some(dir) => dir,
        None => {
            error!("Could not determine flatpak app directory");
            return;
        }
    };

    if !app_dir.exists() {
        info!(
            "Flatpak app directory does not exist yet: {}",
            app_dir.display()
        );
        if let Err(e) = std::fs::create_dir_all(&app_dir) {
            warn!("Failed to create flatpak app directory: {}", e);
        }
    }

    info!("Starting flatpak watcher on: {}", app_dir.display());

    let (tx, rx) = mpsc::channel();

    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        notify::Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to create file watcher: {}", e);
            return;
        }
    };

    if let Err(e) = watcher.watch(&app_dir, RecursiveMode::NonRecursive) {
        error!("Failed to watch directory: {}", e);
        return;
    }

    *WATCHER.lock().await = Some(watcher);

    let mut last_event = Instant::now();
    let debounce = Duration::from_secs(2);

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => {
                let dominated = matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Remove(_)
                );

                if dominated && last_event.elapsed() >= debounce {
                    last_event = Instant::now();
                    debug!("Flatpak app directory changed, emitting signal");
                    emit_installed_apps_updated().await;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                error!("Watcher channel disconnected");
                break;
            }
        }
    }
}

async fn emit_installed_apps_updated() {
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to get session connection for signal: {}", e);
            return;
        }
    };

    let object_path = match zbus::zvariant::ObjectPath::try_from(CLIENT_PATH) {
        Ok(p) => p,
        Err(e) => {
            error!("Invalid object path: {}", e);
            return;
        }
    };

    let iface_ref = match conn
        .object_server()
        .interface::<_, LibraryProvider>(object_path)
        .await
    {
        Ok(i) => i,
        Err(e) => {
            warn!("Could not get interface for signal emission: {}", e);
            return;
        }
    };

    let emitter = iface_ref.signal_emitter();
    if let Err(e) = LibraryProvider::installed_apps_updated(emitter).await {
        error!("Failed to emit installed_apps_updated signal: {}", e);
    } else {
        info!("Emitted installed_apps_updated signal");
    }
}
