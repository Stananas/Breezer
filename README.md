<div align="center">

# 🎧 Breezer

**Ultra-lightweight, native, open-source Deezer client.**

Linux · macOS · Windows — built with **Rust**, **Slint**, lovingly minimal.

![License](https://img.shields.io/badge/license-AGPL--3.0-blue) ![Rust](https://img.shields.io/badge/Rust-1.85+-orange) ![Platforms](https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey)

</div>

---

## ✨ Features

- 🧊 **Lightweight** — a small binary, a small RAM footprint. No Electron, no bloat.
- 🎨 **Deeply customizable** — manual color overrides, full **theme system** (palette + layout + typography), community themes.
- 🧩 **Modular layout** — dockable panels, drag & drop, saved workspaces (*Listening*, *Browsing*, *Compact*) + your own profiles.
- 🔌 **Plugin-ready** — stable public plugin API from day one (logic + shell extension points).
- 🔎 Deezer search, playlists, favorites, metadata (public API + authenticated API).
- 🎵 Real streaming for free-tier tracks (ARL-based, AES decryption isolated & auditable).
- 🌍 **i18n**: English & French (more languages planned).
- 🔄 **Auto-update** from GitHub Releases, semantic-versioned.
- 🔓 **AGPL-3.0** — free software, always.

## 📸 Screenshots

> 🖼️ **Screenshots needed — community request.**
> We're looking for volunteers to run Breezer on their platform (Linux, macOS, Windows) and share screenshots (default theme + at least one community theme). Each screenshot gets credited in the README and the website.
> See [docs/SCREENSHOTS.md](docs/SCREENSHOTS.md) for the exact list and how to contribute (FR/EN welcome).

*Screenshots will appear here once contributed.*

## 🚀 Quick start (from source)

```bash
# Requirements: Rust stable (1.85+), Linux: libasound2-dev, libxkbcommon-dev, cmake + clang
cd software
cargo run --release
```

First launch: open the app, paste your Deezer **ARL** in the sidebar
(see [docs/ARL.md](docs/ARL.md) — 2 minutes, browser developer tools), then hit *Connect*.

## 📦 Downloads

Pre-built binaries (AppImage/deb, dmg, msi/nsis + portable archives) are published on the
[GitHub Releases](https://github.com/Breezer-App/breezer/releases) page, with auto-update support.

## 🧱 Repository layout

```
├── software/   # The Rust client (Cargo workspace: breezer + breezer-plugin-api)
├── site/       # The website: Bun monolith (Next.js + Elysia.js)
├── themes/     # Official + community theme manifests (marketplace)
├── docs/       # ARL, themes, plugins, screenshots…
└── .github/workflows/  # CI, versioning (release-plz), releases (cargo-packager)
```

## 🛠 Development

```bash
cargo check -p breezer            # quick check
cargo run -- --selftest           # audio output self-test (no GUI)
cargo run -p breezer              # run the app
cargo test -p breezer
```

See [docs/THEMES.md](docs/THEMES.md) (create & submit a theme),
[docs/PLUGINS.md](docs/PLUGINS.md) (plugin API), [docs/SCREENSHOTS.md](docs/SCREENSHOTS.md).

## 🌍 Website

`site/` is a monolithic Bun server: **Next.js** (App Router, FR/EN via `next-intl`) + **Elysia.js** (`/api/*` — themes marketplace proxy, release metadata, download stats). See `site/README.md`.

## ⚖️ License & legal

**AGPL-3.0-or-later**. Breezer is an independent, third-party client. It is **not** affiliated with
Deezer SAS. Streaming depends on your Deezer account rights; the app never bypasses DRM
(Widevine-protected content is not supported, by design). Use ethically, in accordance with
Deezer's Terms of Service and your local law.

---

**Made with ❤️ and Rust.** Contributions, themes, translations and translations of this README are welcome!