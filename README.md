# Flatpak Plugin for Playtron GameOS

Table of Contents:
- [Introduction](#introduction)
- [Build](#build)
- [Installation](#installation)
- [Configuration](#configuration)
- [Filters](#filters)
- [Caching](#caching)

## Introduction

This plugin provides [Flathub](https://flathub.org/) catalog integration for [Playtron GameOS](https://github.com/playtron-os/gameos).
It registers as a D-Bus plugin (`one.playtron.plugin.flatpak`) with playserve and exposes the Flathub catalog as a browsable library,
with support for searching, installing, and uninstalling Flatpak applications.

## Build

Building is optional and recommended for plugin developers only.

A basic build can be done with `cargo build`. Copy the plugin to your device:
`scp target/debug/playtron-plugin-flatpak playtron@$DEVICE_IP:~/.local/share/playtron/plugins/flatpak/`

Fully automated binary tarball and RPM builds are supported with the use of a container. Both x86_64 and aarch64 CPU architectures are supported. These commands will output packages to the `dist` directory.

```shell
make in-docker TARGET='dist'
```
```shell
make in-docker TARGET='dist' ARCH="aarch64"
```

## Installation

Install the plugin system-wide:

```shell
sudo make install
```

This installs the binary to `/usr/bin/playtron-plugin-flatpak` and the default configuration to `/usr/share/playtron/plugins/flatpak/`.

Restart the `playserve` service to load the plugin:

```shell
systemctl --user restart playserve
```

## Configuration

The plugin reads configuration from two paths (first match wins):

1. **User config:** `~/.local/share/playtron/plugins/flatpak/config.yaml`
2. **System config:** `/usr/share/playtron/plugins/flatpak/config.yaml`

If neither exists, built-in defaults are used.

```yaml
cache_days: 30  # 0 to disable caching entirely

filters:
  verified_only: true
  games_only: true
  controller_support: true
  touchscreen_support_mandatory: false
  foss_only: false
  # app_allowlist:
  #   - com.play0ad.zeroad
  #   - org.luanti.luanti
```

## Filters

Filters control which apps from the Flathub catalog appear in the library. All filter data is cached locally so changing filters takes effect immediately without re-fetching.

| Filter | Default | Description |
|---|---|---|
| `verified_only` | `true` | Only show apps verified by their developer on Flathub |
| `games_only` | `true` | Only show apps categorized as games |
| `controller_support` | `true` | Only show apps whose metadata mentions gamepad/controller support (heuristic) |
| `touchscreen_support_mandatory` | `false` | Only show apps whose AppStream metadata recommends touch input |
| `foss_only` | `false` | Only show apps with a FOSS license (excludes proprietary) |
| `app_allowlist` | `[]` (empty) | When set, only listed app IDs appear in the catalog |

Installed Flatpak apps always appear in the library regardless of filter settings.

## Caching

The plugin caches Flathub data locally to avoid long startup times:

| File | Contents |
|---|---|
| `catalog_cache.json` | All Flathub app IDs with name, verified status, game classification, and controller support |
| `metadata_cache.json` | Full item metadata (description, screenshots, developers) for all apps |

Cache files are stored in `~/.local/share/playtron/plugins/flatpak/` and expire after `cache_days` days (default 30). Set `cache_days: 0` to disable caching entirely.

To force a cache refresh, delete the cache files and restart:

```shell
rm ~/.local/share/playtron/plugins/flatpak/*_cache.json
systemctl --user restart playserve
```
