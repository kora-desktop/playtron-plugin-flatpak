use log::{debug, error, info};
use std::process::Stdio;
use tokio::process::{Child, Command};

use crate::types::results::ResultWithError;

#[derive(Debug, Clone)]
pub struct InstalledFlatpak {
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub size: String,
    #[allow(dead_code)]
    pub origin: String,
    #[allow(dead_code)]
    pub arch: String,
}

pub struct FlatpakClient;

#[allow(dead_code)]
impl FlatpakClient {
    pub async fn ensure_flathub_remote() -> ResultWithError<()> {
        info!("Ensuring flathub remote is configured");
        let output = Command::new("flatpak")
            .args([
                "remote-add",
                "--user",
                "--if-not-exists",
                "flathub",
                "https://dl.flathub.org/repo/flathub.flatpakrepo",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to add flathub remote: {}", stderr);
        }
        Ok(())
    }

    pub async fn list_installed() -> ResultWithError<Vec<InstalledFlatpak>> {
        debug!("Listing installed user flatpaks");
        let output = Command::new("flatpak")
            .args([
                "list",
                "--user",
                "--app",
                "--columns=application,name,version,size,origin,arch",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Failed to list installed flatpaks: {}", stderr);
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let apps: Vec<InstalledFlatpak> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let cols: Vec<&str> = line.split('\t').collect();
                if cols.len() >= 4 {
                    Some(InstalledFlatpak {
                        app_id: cols[0].to_string(),
                        name: cols.get(1).unwrap_or(&"").to_string(),
                        version: cols.get(2).unwrap_or(&"").to_string(),
                        size: cols.get(3).unwrap_or(&"").to_string(),
                        origin: cols.get(4).unwrap_or(&"").to_string(),
                        arch: cols.get(5).unwrap_or(&"").to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(apps)
    }

    pub async fn install(app_id: &str) -> ResultWithError<Child> {
        info!("Installing flatpak: {}", app_id);
        let child = Command::new("flatpak")
            .args(["install", "--user", "-y", "flathub", app_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(child)
    }

    pub async fn uninstall(app_id: &str) -> ResultWithError<()> {
        info!("Uninstalling flatpak: {}", app_id);
        let output = Command::new("flatpak")
            .args(["uninstall", "--user", "-y", app_id])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to uninstall {}: {}", app_id, stderr).into());
        }
        Ok(())
    }

    pub async fn update(app_id: &str) -> ResultWithError<Child> {
        info!("Updating flatpak: {}", app_id);
        let child = Command::new("flatpak")
            .args(["update", "--user", "-y", app_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(child)
    }

    pub async fn info(app_id: &str) -> ResultWithError<String> {
        debug!("Getting info for flatpak: {}", app_id);
        let output = Command::new("flatpak")
            .args(["info", "--user", app_id])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to get info for {}: {}", app_id, stderr).into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn is_installed(app_id: &str) -> bool {
        Command::new("flatpak")
            .args(["info", "--user", app_id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }
}
