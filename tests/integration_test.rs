use playtron_plugin_flatpak::flatpak::catalog::CatalogCache;
use playtron_plugin_flatpak::flatpak::client::FlatpakClient;
use playtron_plugin_flatpak::flatpak::service::select_best_screenshot;
use playtron_plugin_flatpak::flathub::api::FlathubClient;

const TEST_APP: &str = "org.gnome.Calculator";

#[tokio::test]
#[ignore]
async fn test_appstream_screenshot_selects_image() {
    let client = FlathubClient::new();
    let appstream = client.get_appstream(TEST_APP).await.unwrap();
    let images = select_best_screenshot(&appstream.screenshots);
    assert_eq!(images.len(), 1, "Should select exactly one image");
    let img = &images[0];
    assert_eq!(img.image_type, "landscape");
    assert_eq!(img.source, "flatpak");
    assert!(
        img.url.starts_with("https://dl.flathub.org/"),
        "Image URL should be from Flathub CDN, got: {}",
        img.url
    );
}

#[tokio::test]
#[ignore]
async fn test_verified_collection_pagination() {
    let client = FlathubClient::new();
    let page1 = client.get_verified_collection(1, 25).await.unwrap();
    assert!(!page1.hits.is_empty());
    assert!(page1.total_pages.unwrap_or(0) > 1);

    let page2 = client.get_verified_collection(2, 25).await.unwrap();
    assert!(!page2.hits.is_empty());
    assert_ne!(page1.hits[0].app_id, page2.hits[0].app_id);
}

#[tokio::test]
#[ignore]
async fn test_fetch_all_verified_includes_0ad() {
    let cache = CatalogCache::fetch_fresh().await.unwrap();
    eprintln!("Total verified apps fetched: {}", cache.items.len());
    assert!(
        cache.items.len() > 1000,
        "Should fetch more than 1000 apps (Meilisearch cap), got {}",
        cache.items.len()
    );
    assert!(
        cache.items.iter().any(|e| e.app_id == "com.play0ad.zeroad"),
        "0 A.D. (com.play0ad.zeroad) should be in the catalog"
    );
}

#[tokio::test]
#[ignore]
async fn test_search_flathub() {
    let client = FlathubClient::new();
    let results = client.search("calculator").await.unwrap();
    assert!(!results.is_empty(), "Search should return results");
    assert!(
        results.iter().any(|h| h.app_id == TEST_APP),
        "Search results should contain {}",
        TEST_APP
    );
}

#[tokio::test]
#[ignore]
async fn test_install_and_uninstall() {
    FlatpakClient::ensure_flathub_remote().await.unwrap();

    let mut child = FlatpakClient::install(TEST_APP).await.unwrap();
    let status = child.wait().await.unwrap();
    assert!(status.success(), "Install should succeed");

    let installed = FlatpakClient::list_installed().await.unwrap();
    assert!(
        installed.iter().any(|a| a.app_id == TEST_APP),
        "{} should be in installed list after install",
        TEST_APP
    );

    FlatpakClient::uninstall(TEST_APP).await.unwrap();

    let installed = FlatpakClient::list_installed().await.unwrap();
    assert!(
        !installed.iter().any(|a| a.app_id == TEST_APP),
        "{} should not be in installed list after uninstall",
        TEST_APP
    );
}
