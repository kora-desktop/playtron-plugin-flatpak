# Contributing

Table of Contents:
- [Adding a New Filter](#adding-a-new-filter)
- [Metadata Cache](#metadata-cache)
- [Troubleshooting](#troubleshooting)

## Adding a New Filter

Filters restrict which Flathub apps appear in the catalog. Each filter is a boolean cached per app in `catalog_cache.json` and toggled in `config.yaml`. Adding a new filter touches five files:

### 1. Data source (`src/flathub/types.rs`)

If the filter needs data not already in `AppstreamInfo` or `SearchHit`, add the field here. Use `#[serde(default)]` for fields that may be missing or null in the API response. If a field can be either a value or null, use `Option<T>` rather than `T`.

The Flathub API docs live at `https://flathub.org/api/v2/docs`. Test responses with:

```shell
curl -s https://flathub.org/api/v2/appstream/org.example.App | jq .
```

### 2. Cache entry (`src/flatpak/catalog.rs`)

Add a `bool` field to `CatalogEntry` (e.g. `pub has_feature: bool`). This gets serialized into `catalog_cache.json`.

Add a detection helper function near the top of the file alongside `is_game_category` and `has_controller_keywords`.

Populate the field in `fetch_all_apps()` in two places:
- **Collection entries** (the first ~1000 verified apps from `/collection/verified`): these don't include full appstream data, so you may need a parallel enrichment pass (see `controller_support` for an example).
- **Remaining entries** (fetched individually via `/appstream/{id}`): add the check inline where `is_game` and `has_controller_support` are set.

### 3. Config (`src/config.rs`)

Add the field to `Filters` with `#[serde(default = "default_true")]` or `#[serde(default)]` depending on the desired default. Update the manual `Default` impl to match.

### 4. Filter application (`src/flatpak/catalog.rs`)

Add a `.filter()` line in `to_provider_items()`:

```rust
.filter(|entry| !filters.my_filter || entry.has_feature)
```

### 5. Tests (`tests/integration_test.rs`)

- Add the new field to every `Filters` struct literal in existing tests.
- Assert that a known app has the expected value (pick stable, well-known apps like VLC or 0 A.D.).
- Test that the filter excludes/includes the expected apps.

Integration tests hit the live Flathub API and are `#[ignore]`'d by default:

```shell
make test-integration
```

### 6. Config file and docs

Add the filter to `config.yaml` and update the filters table in `README.md`.

### Cache compatibility

Adding a new field to `CatalogEntry` makes old `catalog_cache.json` files incompatible (deserialization will fail and the cache will be re-fetched). This is handled gracefully — `load_from_disk()` returns `None` on parse failure. The `metadata_cache.json` is unaffected by filter changes since its structure is independent.

## Metadata Cache

The `metadata_cache.json` file stores pre-fetched `ItemMetadata` JSON for every app in the catalog. This is what `get_item_metadata()` returns to playserve when it requests details for a specific app.

### How it works

- **Structure:** A `HashMap<String, String>` where keys are app IDs and values are serialized `ItemMetadata` JSON strings.
- **Built by:** `MetadataCache::fetch_all()` in `src/flatpak/catalog.rs`. It fetches `/appstream/{id}` for every app ID in the catalog (up to 50 concurrent requests) and constructs an `ItemMetadata` per app with name, summary, description, screenshots, developers, and provider info.
- **Used by:** `FlatpakService::get_item_metadata()` in `src/flatpak/service.rs`. When playserve requests metadata for an app, the plugin looks it up in the HashMap. If not found (e.g. a newly installed app not in the cache), it falls back to a live API call.
- **TTL:** Same `cache_days` setting as the catalog cache (default 30 days, 0 disables).

### Why it exists

Without this cache, every call to `get_item_metadata()` would make a live HTTP request to the Flathub API. On startup, playserve requests metadata for every app in the library — with ~3000 apps, this caused 30+ minute load times. The metadata cache reduces this to an instant HashMap lookup.

### Relationship to catalog cache

The two caches are built sequentially on startup in `FlatpakService::new()`:

1. `CatalogCache::load_or_fetch()` — gets the list of all apps with filter data
2. `MetadataCache::load_or_fetch()` — fetches full metadata for every app ID from step 1

If the catalog cache fails (e.g. network error), the metadata cache receives an empty app list and produces an empty cache. Both caches share the same `cache_days` TTL and storage directory.

### Modifying metadata fields

To change what's stored in the metadata cache, modify the `ItemMetadata` construction in `MetadataCache::fetch_all()`. The `ItemMetadata` struct is defined in `src/types/app.rs`. Since values are stored as serialized JSON strings, changes to `ItemMetadata` fields don't break deserialization of the outer `MetadataCache` — but the inner JSON will be stale until the cache expires or is manually deleted.

## Troubleshooting

### Running the plugin standalone

Stop the managed instance and run the binary directly to see all log output:

```shell
systemctl --user stop playserve
timeout 60 ~/.local/share/playtron/plugins/flatpak/playtron-plugin-flatpak 2>&1 | tee /tmp/flatpak-plugin.log
```

Or with debug logging:

```shell
RUST_LOG=debug timeout 60 playtron-plugin-flatpak 2>&1 | tee /tmp/flatpak-plugin.log
```

### Viewing logs

The plugin logs to stdout. When managed by playserve, check the journal:

```shell
journalctl --user -u playserve -f
```

Filter for plugin-specific messages:

```shell
journalctl --user -u playserve | grep -i flatpak
```

### Key log messages to look for

| Message | Meaning |
|---|---|
| `Loaded catalog from cache (N items)` | Cache hit — no network requests needed |
| `Catalog cache is stale, refreshing` | Cache expired, re-fetching from Flathub |
| `Fetched N total apps from Flathub` | Fresh catalog fetch completed |
| `Loaded metadata cache from disk (N entries)` | Metadata cache hit |
| `Failed to create cache directory` | Permission issue with `~/.local/share/` |
| `error decoding response body` | API response shape changed — check `types.rs` deserialization |

### Inspecting cache files

```shell
# Cache location
ls -lh ~/.local/share/playtron/plugins/flatpak/*_cache.json

# Count cached apps
cat ~/.local/share/playtron/plugins/flatpak/catalog_cache.json | python3 -m json.tool | grep -c '"app_id"'

# Check a specific app
cat ~/.local/share/playtron/plugins/flatpak/catalog_cache.json | python3 -c "
import json, sys
data = json.load(sys.stdin)
for item in data['items']:
    if 'dolphin' in item['app_id'].lower():
        print(json.dumps(item, indent=2))
"
```

### Forcing a cache refresh

```shell
rm ~/.local/share/playtron/plugins/flatpak/*_cache.json
systemctl --user restart playserve
```

### Useful Flathub API commands

```shell
# Get all app IDs
curl -s https://flathub.org/api/v2/appstream | jq length

# Get appstream data for a specific app
curl -s https://flathub.org/api/v2/appstream/com.play0ad.zeroad | jq '{name, categories, keywords, description}'

# Get verification status
curl -s https://flathub.org/api/v2/verification/com.play0ad.zeroad/status | jq .

# Search for an app
curl -s -X POST https://flathub.org/api/v2/search -H 'Content-Type: application/json' -d '{"query":"calculator"}' | jq '.hits[] | {app_id, name}'
```
