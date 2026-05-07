use playtron_plugin_flatpak::config::Filters;
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
async fn test_fetch_all_apps_includes_verified_and_unverified() {
    let cache = CatalogCache::fetch_fresh().await.unwrap();
    eprintln!("Total apps fetched: {}", cache.items.len());
    assert!(
        cache.items.len() > 2500,
        "Should fetch all Flathub apps, got {}",
        cache.items.len()
    );

    let zero_ad = cache
        .items
        .iter()
        .find(|e| e.app_id == "com.play0ad.zeroad");
    assert!(zero_ad.is_some(), "0 A.D. should be in the catalog");
    let zero_ad = zero_ad.unwrap();
    assert!(zero_ad.verified, "0 A.D. should be marked verified");
    assert!(zero_ad.is_game, "0 A.D. should be marked as a game");

    let vlc = cache
        .items
        .iter()
        .find(|e| e.app_id == "org.videolan.VLC");
    assert!(vlc.is_some(), "VLC should be in the catalog");
    let vlc = vlc.unwrap();
    assert!(!vlc.verified, "VLC should be marked unverified");
    assert!(!vlc.is_game, "VLC should NOT be marked as a game");
    assert!(
        !vlc.has_controller_support,
        "VLC should NOT have controller support"
    );

    let dolphin = cache
        .items
        .iter()
        .find(|e| e.app_id == "org.DolphinEmu.dolphin-emu");
    assert!(dolphin.is_some(), "Dolphin should be in the catalog");
    let dolphin = dolphin.unwrap();
    assert!(
        dolphin.has_controller_support,
        "Dolphin should have controller support (description mentions controllers)"
    );
}

#[tokio::test]
#[ignore]
async fn test_to_provider_items_respects_filters() {
    let cache = CatalogCache::fetch_fresh().await.unwrap();

    let no_filters = Filters {
        verified_only: false,
        games_only: false,
        controller_support: false,
    };
    let verified_only = Filters {
        verified_only: true,
        games_only: false,
        controller_support: false,
    };
    let games_only = Filters {
        verified_only: false,
        games_only: true,
        controller_support: false,
    };
    let both = Filters {
        verified_only: true,
        games_only: true,
        controller_support: false,
    };

    let all_items = cache.to_provider_items(&no_filters);
    let verified_items = cache.to_provider_items(&verified_only);
    let game_items = cache.to_provider_items(&games_only);
    let both_items = cache.to_provider_items(&both);

    let controller_filter = Filters {
        verified_only: false,
        games_only: false,
        controller_support: true,
    };
    let controller_items = cache.to_provider_items(&controller_filter);

    eprintln!(
        "All: {}, Verified: {}, Games: {}, Both: {}, Controller: {}",
        all_items.len(),
        verified_items.len(),
        game_items.len(),
        both_items.len(),
        controller_items.len()
    );

    assert!(
        all_items.len() > verified_items.len(),
        "Unfiltered ({}) should be more than verified-only ({})",
        all_items.len(),
        verified_items.len()
    );
    assert!(
        all_items.len() > game_items.len(),
        "Unfiltered ({}) should be more than games-only ({})",
        all_items.len(),
        game_items.len()
    );
    assert!(
        both_items.len() <= game_items.len(),
        "Both filters ({}) should be <= games-only ({})",
        both_items.len(),
        game_items.len()
    );

    assert!(
        all_items.iter().any(|i| i.id == "org.videolan.VLC"),
        "VLC should appear in unfiltered list"
    );
    assert!(
        !verified_items.iter().any(|i| i.id == "org.videolan.VLC"),
        "VLC should NOT appear in verified-only list"
    );
    assert!(
        !game_items.iter().any(|i| i.id == "org.videolan.VLC"),
        "VLC should NOT appear in games-only list"
    );
    assert!(
        game_items.iter().any(|i| i.id == "com.play0ad.zeroad"),
        "0 A.D. should appear in games-only list"
    );

    assert!(
        all_items.len() > controller_items.len(),
        "Unfiltered ({}) should be more than controller-only ({})",
        all_items.len(),
        controller_items.len()
    );
    assert!(
        controller_items
            .iter()
            .any(|i| i.id == "org.DolphinEmu.dolphin-emu"),
        "Dolphin should appear in controller-support list"
    );
    assert!(
        !controller_items.iter().any(|i| i.id == "org.videolan.VLC"),
        "VLC should NOT appear in controller-support list"
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
