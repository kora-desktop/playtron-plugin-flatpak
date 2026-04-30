use log::{debug, error, info};
use std::process::Stdio;
use tokio::process::{Child, Command};

use crate::types::results::ResultWithError;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InstalledFlatpak {
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub size: String,
    pub origin: String,
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
            .args([
                "install",
                "--user",
                "-y",
                "--noninteractive",
                "flathub",
                app_id,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(child)
    }

    pub async fn install_bundle(path: &str) -> ResultWithError<Child> {
        info!("Installing flatpak bundle: {}", path);
        let child = Command::new("flatpak")
            .args(["install", "--user", "-y", "--noninteractive", "--bundle", path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(child)
    }

    pub async fn install_flatpakref(path: &str) -> ResultWithError<Child> {
        info!("Installing flatpak from ref: {}", path);
        let child = Command::new("flatpak")
            .args(["install", "--user", "-y", "--noninteractive", "--from", path])
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
            .args(["update", "--user", "-y", "--noninteractive", app_id])
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

    pub async fn get_download_size(app_id: &str) -> u64 {
        let output = Command::new("flatpak")
            .args(["remote-info", "--user", "flathub", app_id])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                for line in text.lines() {
                    let trimmed = line.trim();
                    if let Some(value) = trimmed.strip_prefix("Download:") {
                        return parse_size_output(value.trim());
                    }
                }
                0
            }
            _ => 0,
        }
    }
}

fn parse_size_output(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 2 {
        return s.parse::<u64>().unwrap_or(0);
    }
    let num: f64 = parts[0].replace(',', ".").parse().unwrap_or(0.0);
    let unit = parts[1].to_lowercase();
    let multiplier: u64 = match unit.as_str() {
        "kb" => 1_000,
        "mb" => 1_000_000,
        "gb" => 1_000_000_000,
        "kib" => 1_024,
        "mib" => 1_048_576,
        "gib" => 1_073_741_824,
        "bytes" | "b" => 1,
        _ => 1,
    };
    (num * multiplier as f64) as u64
}
