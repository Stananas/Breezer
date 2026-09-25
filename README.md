<p align="right">
<a href="README.fr.md">🇫🇷 <b>Français</b></a> &nbsp;·&nbsp; 🇬🇧 <b>English</b>
</p>

<div align="center">

# 🎧 Breezer

**The ultra-lightweight native Deezer client.** Rust + Slint. No Electron. No bloat.

Linux · macOS · Windows

![License](https://img.shields.io/badge/license-AGPL--3.0%20or%20later-blue)
![Rust](https://img.shields.io/badge/Rust-1.85+-orange)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey)
[![CI](https://img.shields.io/github/actions/workflow/status/Stananas/Breezer/ci.yml?branch=main&label=CI)](https://github.com/Stananas/Breezer/actions/workflows/ci.yml)

</div>

---

## What is Breezer?

A small, fast, native music player for **Deezer** — search, stream, browse your playlists and listening history with a clean, customizable UI. Built around the same auth & streaming flow as [tui-dzr](https://github.com/dunderdoo/tui-dzr) (ARL-based, Blowfish decryption).

## ✨ Key features

- 🧊 **Lightweight** — a few MB binary, small RAM footprint.
- 🎵 **Real streaming** of free-tier tracks (ARL auth, decrypt isolated & auditable).
- 🏠 Deezer-style pages: **Home** (continue listening, recently played shelves), **Explore** (trending), **Favorites**, **Playlists**.
- 🎨 **Theme system** — dark / light / AMOLED built-in, custom color editor, community themes.
- 🧩 **Dockable panels** — drag & drop layout, saved workspaces.
- 🌍 **i18n** — French & English (UI and status messages).
- 🔄 **Self-updating** — checks, downloads and replaces itself automatically; one click to restart.
- 🔓 **AGPL-3.0** — free software.

## 📸 Screenshots

| Dark (default) | Light | AMOLED |
|---|---|---|
| [![Home – dark](assets/screenshots/home-dark.png)](assets/screenshots/home-dark.png) | [![Home – light](assets/screenshots/home-light.png)](assets/screenshots/home-light.png) | [![Home – amoled](assets/screenshots/home-amoled.png)](assets/screenshots/home-amoled.png) |

[![Explore – trending](assets/screenshots/explorer-dark.png)](assets/screenshots/explorer-dark.png)

## 🚀 Quick start

```bash
# Requirements: Rust 1.85+, Linux: libasound2-dev libxkbcommon-dev cmake clang
cd software
cargo run --release
```

First run: paste your Deezer **ARL cookie** (see [docs/ARL.md](docs/ARL.md), ~2 min) and connect.

## 📦 Downloads

Pre-built installers (AppImage/deb, dmg, nsis) are published on the **[Releases](https://github.com/Stananas/Breezer/releases)** page,
with automatic updates.

## 🧱 Repository layout

```
software/   # Rust client (Cargo workspace: breezer + plugin-api)
site/       # Website (Bun: Next.js + Elysia.js)
themes/     # Official & community theme manifests
docs/       # ARL, themes, plugins, screenshots
assets/     # README screenshots & branding
```

## 🛠 Development

```bash
cargo check -p breezer        # quick check
cargo run -p breezer          # run the app (alias: cargo run)
cargo run -- --selftest       # audio self-test, no GUI
cargo test -p breezer
```

Docs: [Themes](docs/THEMES.md) · [Plugins](docs/PLUGINS.md) · [Screenshots](docs/SCREENSHOTS.md) · [Contributing](CONTRIBUTING.md)

## ⚖️ License & legal

**AGPL-3.0-or-later.** Breezer is an independent third-party client, **not affiliated** with Deezer SAS.
Streaming depends on your Deezer account rights; DRM-protected content (Widevine) is never bypassed.
Use it in accordance with Deezer's Terms of Service and your local law.

---

**Made with ❤️ and Rust.** Contributions, themes and translations are welcome — a
[French translation of this README](README.fr.md) is available.