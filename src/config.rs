use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::constants;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    pub filters: Filters,
    #[serde(default = "default_cache_days")]
    pub cache_days: u64,
}

fn default_cache_days() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            filters: Filters::default(),
            cache_days: default_cache_days(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Filters {
    #[serde(default = "default_true")]
    pub verified_only: bool,
    #[serde(default = "default_true")]
    pub games_only: bool,
    #[serde(default)]
    pub controller_support_mandatory: bool,
    #[serde(default)]
    pub touchscreen_support_mandatory: bool,
    #[serde(default)]
    pub foss_only: bool,
    #[serde(default)]
    pub app_allowlist: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            verified_only: true,
            games_only: true,
            controller_support_mandatory: false,
            touchscreen_support_mandatory: false,
            foss_only: false,
            app_allowlist: Vec::new(),
        }
    }
}

fn user_config_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| {
        d.join(constants::USER_CONFIG_SUBDIR)
            .join(constants::CONFIG_FILENAME)
    })
}

fn system_config_path() -> PathBuf {
    PathBuf::from(constants::SYSTEM_CONFIG_PATH)
}

pub fn load_config() -> Config {
    if let Some(user_path) = user_config_path() {
        if user_path.exists() {
            info!("Loading config from user path: {}", user_path.display());
            match fs::read_to_string(&user_path) {
                Ok(contents) => match serde_yaml::from_str(&contents) {
                    Ok(config) => return config,
                    Err(e) => warn!("Failed to parse user config: {}", e),
                },
                Err(e) => warn!("Failed to read user config: {}", e),
            }
        }
    }

    let sys_path = system_config_path();
    if sys_path.exists() {
        info!("Loading config from system path: {}", sys_path.display());
        match fs::read_to_string(&sys_path) {
            Ok(contents) => match serde_yaml::from_str(&contents) {
                Ok(config) => return config,
                Err(e) => warn!("Failed to parse system config: {}", e),
            },
            Err(e) => warn!("Failed to read system config: {}", e),
        }
    }

    info!("No config file found, using defaults");
    Config::default()
}
