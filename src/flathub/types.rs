use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct SearchRequest {
    pub query: String,
}

// Deserialization targets for Flathub API responses — fields populated by serde
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    #[serde(rename = "totalHits")]
    pub total_hits: Option<u64>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct SearchHit {
    pub name: Option<String>,
    pub summary: Option<String>,
    pub app_id: String,
    pub icon: Option<String>,
    pub developer_name: Option<String>,
    pub verification_verified: Option<bool>,
    pub verification_method: Option<String>,
    pub project_license: Option<String>,
    pub is_free_license: Option<bool>,
    pub runtime: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
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

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Screenshot {
    #[serde(default)]
    pub sizes: Vec<ScreenshotSize>,
    pub caption: Option<String>,
    pub default: Option<bool>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ScreenshotSize {
    pub src: String,
    pub width: Option<String>,
    pub height: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Release {
    pub version: Option<String>,
    pub timestamp: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct VerificationStatus {
    pub verified: bool,
    pub method: Option<String>,
    pub timestamp: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct CollectionResponse {
    pub hits: Vec<SearchHit>,
    #[serde(rename = "totalHits")]
    pub total_hits: Option<u64>,
    pub page: Option<u32>,
    #[serde(rename = "totalPages")]
    pub total_pages: Option<u32>,
}
