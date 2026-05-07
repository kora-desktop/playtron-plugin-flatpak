use playtron_plugin_flatpak::flatpak::client::FlatpakClient;
use playtron_plugin_flatpak::flathub::api::FlathubClient;

const TEST_APP: &str = "org.gnome.Calculator";

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
