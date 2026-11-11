# Plan: Convert Local Games Plugin to User Flatpak Plugin

## Context

This plugin currently manages manually sideloaded games by scanning filesystem directories for `gameinfo.yaml` files. It needs to be converted into a Flatpak plugin that manages user-scoped Flatpak installations (`~/.local/share/flatpak`), searches Flathub for apps, and filters based on verification status.

**Key architectural decision**: Flatpak's DBus interfaces (`org.freedesktop.Flatpak.SystemHelper`, `org.freedesktop.portal.Flatpak`) are explicitly not designed for third-party app management. The SystemHelper docs say "not intended to be contacted by third-party applications or libraries" and the portal is only for sandboxed apps managing themselves. Therefore, the plugin will use the `flatpak` CLI with `--user` flag for install/uninstall/list operations, and the Flathub REST API for search and metadata.

---

## Architecture Overview

```
src/
├── main.rs                      (entry point)
├── lib.rs                       (public module exports for integration tests)
├── constants.rs                 (DBus names, paths, API URLs)
├── config.rs                    (YAML config loading with filter definitions)
├── flatpak/
│   ├── mod.rs
│   ├── client.rs               (Flatpak CLI wrapper: install, uninstall, list, info)
│   ├── service.rs              (Business logic combining CLI + API + config)
│   └── watcher.rs              (Monitor ~/.local/share/flatpak/app/ for changes)
├── flathub/
│   ├── mod.rs
│   ├── api.rs                  (Flathub REST API client: search, appstream, verification)
│   └── types.rs                (API request/response serde types)
├── plugin/
│   ├── mod.rs                  (unchanged)
│   ├── dbus.rs                 (update service type reference)
│   ├── errors.rs               (unchanged)
│   ├── plugin_interface.rs     (unchanged - reads from constants.rs)
│   └── library_provider.rs     (rewrite method implementations)
├── types/
│   ├── mod.rs                  (unchanged)
│   ├── app.rs                  (minor adjustments)
│   ├── results.rs              (unchanged)
│   └── cloud_sync.rs           (unchanged)
├── auth/
│   ├── mod.rs                  (unchanged)
│   └── cryptography.rs         (update service type reference)
└── utils/
    ├── mod.rs                  (remove disks/system, keep date)
    └── date.rs                 (unchanged)

tests/
└── integration_test.rs         (search, install, uninstall end-to-end)
```

---

## Configuration File

**System default**: `/usr/share/playtron/plugins/flatpak/config.yaml` (installed by RPM)
**User override**: `~/.local/share/playtron/plugins/flatpak/config.yaml` (overrides system entirely when present)

```yaml
filters:
  verified_only: true
  # Future filters (all individually toggleable):
  # foss_only: false
  # games_only: true
  # controller_support: true
  # touchscreen_support: true
  # allow_emulators: false
```

Config loading priority: user file takes precedence. If user file exists, system file is ignored entirely (no merging). If neither exists, defaults are used and a user config file is created.

---

## Implementation Steps

### 1. Update Project Identity

**Files**: `Cargo.toml`, `constants.rs`, `pluginmanifest.json`, `Makefile`, `.releaserc.yaml`

- Rename package to `playtron-plugin-flatpak`
- DBus bus name: `inc.kora.plugin.flatpak`
- Object path: `/inc/kora/plugin/flatpak/PluginClient0`
- Plugin ID: `"flatpak"`, Name: `"Flatpak"`
- Remove unused deps: `fs_extra`, `sysinfo`, `regex`, `async-stream`, `tokio-util`, `parking_lot`
- Update RPM assets in `Cargo.toml` `[package.metadata.generate-rpm]`:
  - Binary: `playtron-plugin-flatpak` → `/usr/bin/`
  - Manifest: `pluginmanifest.json` → `/usr/share/playtron/plugins/flatpak/`
  - Config: `config.yaml` → `/usr/share/playtron/plugins/flatpak/` (system default)
- Add a `config.yaml` at project root for the default config shipped by RPM

### 2. Create Configuration Module

**File**: `src/config.rs`

- `Config` struct with `Filters` field
- `Filters` struct: `verified_only: bool` (default: `true`)
- `load_config()` checks user path first (`~/.local/share/playtron/plugins/flatpak/config.yaml`), falls back to system path (`/usr/share/playtron/plugins/flatpak/config.yaml`), then hardcoded defaults
- User config overrides system entirely (no merging)
- Extensible design: future filters add new bool/vec fields with serde defaults
- Also create `config.yaml` at project root (shipped by RPM as system default)

### 3. Create Flathub API Client

**Files**: `src/flathub/mod.rs`, `src/flathub/api.rs`, `src/flathub/types.rs`

Endpoints used:
- `POST https://flathub.org/api/v2/search` - search for apps
- `GET https://flathub.org/api/v2/appstream/{app_id}` - app metadata (name, description, screenshots)
- `GET https://flathub.org/api/v2/verification/{app_id}/status` - check if verified/official

`FlathubClient` struct with async methods: `search()`, `get_appstream()`, `get_verification_status()`

Search results include `verification_verified` field to filter by official status.

### 4. Create Flatpak CLI Client

**File**: `src/flatpak/client.rs`

All operations use `tokio::process::Command` with `--user` flag:
- `ensure_flathub_remote()` - `flatpak remote-add --user --if-not-exists flathub ...`
- `list_installed()` - `flatpak list --user --app --columns=application,name,version,size,origin,arch`
- `install(app_id)` - `flatpak install --user -y flathub {app_id}` (returns Child for progress)
- `uninstall(app_id)` - `flatpak uninstall --user -y {app_id}`
- `update(app_id)` - `flatpak update --user -y {app_id}`
- `info(app_id)` - `flatpak info --user {app_id}`
- `is_installed(app_id)` - check via exit code

### 5. Create Service Layer

**File**: `src/flatpak/service.rs`

`FlatpakService` combines FlathubClient + FlatpakClient + Config:
- `search(query)` - search Flathub, apply verified_only filter
- `get_installed_apps()` - list via CLI, map to InstalledApp structs
- `get_provider_items()` - installed apps as ProviderItems
- `get_item_metadata(app_id)` - fetch from Flathub API, return ItemMetadata JSON
- `get_launch_options(app_id)` - returns `flatpak run --user {app_id}`
- `install(app_id)` - spawn CLI, emit progress signals
- `uninstall(app_id)` - run CLI
- `update(app_id)` - spawn CLI with progress
- `refresh()` - re-enumerate installed, emit LibraryUpdated

### 6. Rewrite LibraryProvider

**File**: `src/plugin/library_provider.rs`

Key method mappings:
- `GetInstalledApps()` → `service.get_installed_apps()`
- `GetProviderItems()` → `service.get_provider_items()`
- `Install(app_id, _, _)` → `service.install(app_id)` with signal emission
- `Uninstall(app_id)` → `service.uninstall(app_id)`
- `Update(app_id)` → `service.update(app_id)`
- `GetAppMetadata(app_id)` → `service.get_item_metadata(app_id)` (Flathub API)
- `GetLaunchOptions(app_id)` → `[LaunchOption { exe: "flatpak", args: "run --user {app_id}" }]`
- `Refresh()` → `service.refresh()` with LibraryUpdated signal
- `MoveItem` / `CancelMoveItem` → return error (not applicable)
- `PreLaunchHook` → emit LaunchReady immediately

Property: `InstallTarget` = `"flatpak"` (flatpak manages its own paths)

### 7. Create Watcher

**File**: `src/flatpak/watcher.rs`

- Watch `~/.local/share/flatpak/app/` with `notify` crate
- On Create/Remove events, debounce 2s, emit `InstalledAppsUpdated` signal
- Much simpler than current watcher (no drive monitoring)

### 8. Update Entry Point

**File**: `src/main.rs`

```rust
async fn main() {
    // ... logging ...
    FlatpakClient::ensure_flathub_remote().await?;
    let service = FlatpakService::new();
    build_connection(service).await?;
    register_plugin().await;
    tokio::spawn(flatpak::watcher::start_watcher());
    pending::<()>().await;
}
```

### 9. Delete Old Code

Remove entirely:
- `src/local/` (mod.rs, connector.rs, service.rs, watcher.rs)
- `src/utils/system.rs`, `src/utils/disks.rs`

### 10. Integration Tests

**File**: `tests/integration_test.rs`

```rust
#[tokio::test]
#[ignore] // requires network + flatpak CLI
async fn test_search_install_uninstall() {
    // 1. Search Flathub for "calculator"
    // 2. Verify org.gnome.Calculator in results
    // 3. Install it with --user
    // 4. Verify in installed list
    // 5. Uninstall
    // 6. Verify removed
}
```

Also add `src/lib.rs` to export modules for test access.

### 11. Update Docs and CI

- Rewrite `README.md` for Flatpak plugin usage
- Add `test-integration` Makefile target

---

## Signal Flow

**Install**: Playserve calls Install → plugin emits InstallStarted → spawns `flatpak install --user` → emits InstallProgressed → emits InstallCompleted/Failed → watcher emits InstalledAppsUpdated

**Launch**: Playserve calls GetLaunchOptions → gets `flatpak run --user {app_id}` → calls PreLaunchHook → plugin emits LaunchReady → Playserve executes command

---

## Design Decisions

- **Search is internal**: Playserve handles store discovery separately. The plugin's search capability (FlathubClient) is used internally and by integration tests. Playserve calls `Install(app_id)` with an app_id it already knows about.
- **No custom DBus search method**: The plugin exposes only the standard LibraryProvider interface.

## Verification Plan

1. `cargo clippy` passes with no warnings
2. `cargo build` succeeds with no errors
3. `cargo test` passes (unit tests)
4. Run the plugin manually, verify it registers on DBus: `busctl --user list | grep inc.kora.plugin.flatpak`
5. Call methods via busctl to verify responses:
   - `busctl --user get-property inc.kora.plugin.flatpak /inc/kora/plugin/flatpak/PluginClient0 inc.kora.plugin.Plugin Id`
   - `busctl --user call inc.kora.plugin.flatpak /inc/kora/plugin/flatpak/PluginClient0 inc.kora.plugin.LibraryProvider GetInstalledApps`
6. Run integration tests: `cargo test -- --ignored` (requires flatpak + network)
7. Verify config file is created and respected (toggle verified_only, observe filtered results)

All code must pass `cargo clippy` before being considered complete.
