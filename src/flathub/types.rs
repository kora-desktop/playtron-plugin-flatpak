use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct SearchRequest {
    pub query: String,
}

#[derive(Deserialize, Debug)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    #[serde(rename = "totalHits")]
    #[allow(dead_code)]
    pub total_hits: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct SearchHit {
    pub name: Option<String>,
    #[allow(dead_code)]
    pub summary: Option<String>,
    pub app_id: String,
    #[allow(dead_code)]
    pub icon: Option<String>,
    #[allow(dead_code)]
    pub developer_name: Option<String>,
    pub verification_verified: Option<bool>,
    #[allow(dead_code)]
    pub verification_method: Option<String>,
    #[allow(dead_code)]
    pub project_license: Option<String>,
    #[allow(dead_code)]
    pub is_free_license: Option<bool>,
    #[allow(dead_code)]
    pub runtime: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct AppstreamInfo {
    pub name: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub id: Option<String>,
    pub icon: Option<String>,
    pub developer_name: Option<String>,
    pub project_license: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    pub screenshots: Option<Vec<Screenshot>>,
    #[serde(default)]
    pub releases: Vec<Release>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Screenshot {
    #[serde(default)]
    pub sizes: Vec<ScreenshotSize>,
    pub caption: Option<String>,
    pub default: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ScreenshotSize {
    pub src: String,
    pub width: Option<String>,
    pub height: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Release {
    pub version: Option<String>,
    pub timestamp: Option<u64>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct VerificationStatus {
    pub verified: bool,
    pub method: Option<String>,
    pub timestamp: Option<String>,
}
