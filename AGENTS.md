# AGENTS.md — Breezer project briefing

Everything an agent needs to work on this repo without re-discovering what took
many sessions to learn. Read this first. Repo: **github.com/Stananas/Breezer**
(public, AGPL-3.0-or-later). User speaks French; commit messages can be FR.

---

## 1. What this is

Ultra-lightweight **native Deezer client** (Rust + Slint). No Electron.
Working stack: `software/breezer` = the app (Rust, tokio, reqwest, rodio);
`software/plugin-api` = plugin API crate; `site/` = Bun website (Next.js +
Elysia) non-functional scaffold; `themes/` = theme manifests; `docs/`,
`packaging/` (AUR), `assets/` (README screenshots).

Key links:
- Repo: `https://github.com/Stananas/Breezer`
- Releases live under tags **`breezer-vX.Y.Z`** (see §5).
- Current version: `Cargo.toml` (repo root workspace) → `[workspace.package] version`.
  The Cargo workspace is at the REPO ROOT (members: `software/breezer`,
  `software/plugin-api`) — keep it that way: release-plz `git_only` needs the
  workspace manifest at the git root.
- Branding: violet `#a238ff`, font **Lexend** (embedded), icons in
  `software/breezer/src/ui/icons/`.

## 2. Build / run / test

```bash
# Arch (the dev machine): alsa-lib libxkbcommon fontconfig cmake clang (runtime libs)
cd software
cargo run                # run the app (release: cargo run --release)
cargo build -p breezer
cargo clippy --workspace -- -D warnings   # CI enforces -D warnings + cargo fmt
cargo fmt --all -- --check
cargo test --workspace
cargo run -- --stream-test <id>   # streaming diagnostic (no GUI)
cargo run -- --playlists-test     # playlists diagnostic
```

The dev machine is **Arch Linux + Hyprland** (user "Stananas"). There IS a
session D-Bus in the sandbox (`DBUS_SESSION_BUS_ADDRESS`), which is how MPRIS
is tested (§8).

## 3. Architecture (app.rs services loop)

Two tokio tasks around the Slint event loop:

- **UI → services**: `Cmd` enum over `cmd_tx/cmd_rx` (128 cap).
  Commands are **not Send-heavy**; the loop is `tokio::select!` over
  `{ cmd_rx.recv(), stream_rx.recv(), tick.tick() }`.
- **services → UI**: `Evt` enum over `evt_tx/evt_rx`; a UI-bridge task
  `invoke_from_event_loop` → `apply_event(&window, evt, covers)` on the UI
  thread (threading rule: `slint::Image` is NOT Send → images are built on the
  UI thread from shared decoded-RGBA caches).

**Performance invariant (do not regress):** the services loop must NEVER block
on network. Everything slow runs in `tokio::spawn` tasks; results come back
either via `stream_rx` (streaming bytes) or by writing shared caches:
- `last_cards`, `recent_cards`, `playlists_cache` = `Arc<Mutex<Vec<_>>>`.
- Streaming: `request_stream()` spawns fetch+decrypt; finished bytes arrive on
  `stream_rx` where the loop feeds `rodio` (`player.play_mp3_bytes`). This is
  why navigation stays instant even during buffering.
- Shared caches must not hold a `MutexGuard` across `.await` (future is not
  Send) — clone then release then send.

## 4. Deezer API / streaming (tui-dzr-compatible, do NOT "improve" blindly)

- Auth: ARL stored in `~/.config/breezer/config.json` (secret — never print it).
  Gateway `gw-light.php`: token lives in **`results.checkForm`** (not `TOKEN`),
  POST `{}` JSON, Chrome UA required. Reject `user_id==0 && username==""`.
  Session fields: `SESSION_ID`, `license_token` (session-scoped, refreshed at
  boot).
- Streaming (free-tier): `pageTrack{sng_id}` → `TRACK_TOKEN` → POST
  `media.deezer.com/v1/get_url` (`license_token`, `BF_CBC_STRIPE`, `MP3_128`)
  → signed CDN URL → encrypted bytes → **Blowfish-CBC stripe** decrypt:
  key = xor(md5hex(track_id), `g4el58wc0zvf9na1`), IV `0..7`, 2048-byte chunks,
  keep only chunks where `index % 3 == 0`. Tests match tui-dzr vectors.
- `pageProfile tab=playlists` → user playlists; **cover field is
  `PLAYLIST_PICTURE`** (hash → `e-cdns-images.dzcdn.net/images/cover/{h}/250…`).
  IDs often come as **strings** — parse both `as_u64` and `as_str`.
- `pageProfile tab=history` → listening history (`TAB.history.data`, newest
  first); `parse_history_track` centralizes item parsing (SNG_TITLE, ART_NAME,
  ALB_PICTURE). `recent_played(limit)` / `last_played()` (first).
- Chart: `api.deezer.com/chart/0/tracks` (public). Search: `/search?q=`.

## 5. Versioning / releases — fully automatic (no manual steps)

Pipeline (auto):
- push main (`software/**`) → `ci.yml` (fmt + clippy -D warnings + test +
  themes) AND `version.yml` → `scripts/release-bump.sh`: if there are `feat*`/
  `fix*` Conventional Commits since the last `breezer-v*` tag, it bumps the
  workspace version in the root `Cargo.toml` (feat → minor, fix → patch),
  commits `chore(release): vX.Y.Z`, creates the **annotated tag `breezer-vX.Y.Z`**
  and pushes main + tag. No PR, no release-plz.
- tag `breezer-v*` / `v*` → `release.yml`: cargo-packager installers on 3 OS +
  **SLSA attestation** + SHA256SUMS → GitHub Release (assets auto-updated).

Notes / gotchas:
- The workspace Cargo manifest lives at the **repo root** (`Cargo.toml`,
  members `software/breezer`, `software/plugin-api`) — required for the
  tooling; keep it there. `.gitignore` must cover `target/` at the ROOT
  (a `git add -A` after moving the workspace would otherwise commit build
  artifacts and GitHub rejects the push via LFS limits).
- `release-plz` was used first but its `release-pr` is buggy for
  `git_only` + subdir workspaces (worktree `cargo metadata` errors) — do NOT
  reintroduce it for the PR step. The manual one-shot release path is
  `release-plz release --manifest-path Cargo.toml --git-token "$(gh auth token)"` from the repo root (git_only config at root).
- Updater picks releases whose tag starts with `breezer-v` (§6).

## 6. Auto-update (the app replaces itself)

`src/updater.rs`: at boot `check()` lists GitHub releases and picks the one
whose tag starts with `breezer-v` (legacy `v` fallback, **never** the plugin
release). `pick_binary()` selects the raw asset named **`breezer-{platform_key}`**
(e.g. `breezer-linux-x86_64`; AppImage fallback on Linux). Downloads stream to
`~/.config/breezer/updates/breezer-{version}` + marker `ready.txt`, after an
ELF+size sanity check. UI shows an "Install & restart" button (sidebar);
`apply()` atomically renames over the running exe (Linux/macOS) then relaunches;
a pending download is applied automatically on next boot.

## 7. Config & data (Linux)

`~/.config/breezer/` = `config.json` (ARL secret, volume, zoom, language,
theme_id, theme_overrides, time_format, onboarding_done, zoom), `layout.json`
(dock layout), `updates/` (self-update). Never print the ARL/token.

## 8. MPRIS / media widgets (the hotbar fix)

`src/mpris.rs` (Linux-only): exposes **`org.mpris.MediaPlayer2.breezer`** on the
session bus so desktop widgets (Waybar media, KDE Plasma, GNOME, "dankmaterial"
hotbars…) show Breezer exactly like Brave/other players. It does NOT run on the
multithread runtime: `LocalServer` (zbus) is **not Send**, so it runs on its own
`current_thread` tokio runtime inside a dedicated thread. The loop pushes
`MprisMsg::Status/Track/Position/Volume`; controls (Play/Pause/Next/Previous/
Seek/Volume/Stop) are forwarded as `Cmd` back to the player. Verify with:
`busctl --user list | grep -i mpris` and `busctl --user get-property
org.mpris.MediaPlayer2.breezer /org/mpris/MediaPlayer2
org.mpris.MediaPlayer2.Player PlaybackStatus`.

## 9. UI — Slint 1.18 (hard-won gotchas)

Files: `src/ui/main_window.slint` (exported `MainWindow` = the Window; `DockShell`
root; drag/drop zones), `sidebar.slint`, `content.slint` (views: search/home/
explore/playlists/playlist-detail/settings), `player_bar.slint`, `elements.slint`
(widgets + `Palette`/`TrackInfo`/`PlaylistInfo` structs), `queue_panel.slint`.

**Slint 1.18 gotchas (iterate VISUALLY via `slint-viewer --screenshot` only):**
1. **Free-positioned children without a layout are CENTERED by default** — the
   codebase deliberately uses **absolute x/y/width/height** everywhere to
   avoid it (sidebar rows, player bar, nav).
2. **A brace-less `if cond:` gates ONLY its first child**; subsequent
   indented elements are instantiated independently. Always wrap multiple
   children in a single container element.
3. **`alignment: center` on a layout overrides `stretch` spacers** — the
   spacer won't push content to the end. Drop `alignment` where a spacer
   should absorb space.
4. **`visible: false` still consumes layout space** if the element has an
   explicit size — mask slots with `width: cond ? Npx : 0px`.
5. **Keyboard**: only `FocusScope` gives reliable shortcuts (`forward-focus`
   needs a FOCUSABLE target — use a tiny 1x1 `TextInput` sink; `capture-key-pressed`
   runs before the focused child; return `accept`/`reject`). `KeyEvent` has only
   `text`, `modifiers` (+`repeat`) — no key code. Global zoom = Ctrl+ ±/0 via
   this FocusScope (window resize + theme spacing/radius density; Slint 1.18 has
   NO scale transform / dynamic scale factor).
6. `Text` has no `text-transform`. Element ids must be unique per component.
7. `@image-url` accepts PNG only in-app (no SVG runtime); icons are white PNGs
   recolored with `Image.colorize`. Current set: navigate/player icons incl.
   official Deezer SVGs (in `icons/deezer-*.svg`).
8. `Flickable` in a layout: use `width:100% + vertical-stretch:1`, never both
   `height:100%` and stretch.

**i18n**: two channels, keep in sync — Slint UI `@tr("English default")`
(+ French in `translations/fr/LC_MESSAGES/breezer.po`, bundled at build time),
and Rust-side messages via `i18n.t("key")` (`src/i18n/en.json` + `fr.json`,
both must receive new keys). Status strings must be i18n'd, not hard-coded.

**Dock layout**: left slot always "sidebar"; right slot hidden (width:0 mask).
Panels are conditionally instantiated; do not reintroduce `visible`-based
slot hiding (§9.4).

## 10. Headless verification (the sandbox has no WM)

- GUI can't run interactively; use **`slint-viewer --screenshot out.png
  wrapper.slint`** with a wrapper that imports `MainWindow` and sets
  `current-view`, palette, models. TrackInfo literals require the
  `duration-label` field (added later — wrappers must include it).
- Pixel checks: PIL analysis (background colors: dark bg `#0f1012`, surface
  `#17181c`, surface2 `#1f2127`, primary `#a238ff`).
- Release/MPRIS realness is testable (session bus + GitHub Actions).

## 11. Repo hygiene / conventions

- Commits: Conventional (FR body ok). Never commit secrets/ARL. Never print the
  user's ARL or the `~/.config/breezer/config.json` contents.
- AUR packaging lives in `packaging/aur/` (PKGBUILD + .SRCINFO + .desktop);
  `scripts/sync-package-manifests.sh` refreshes pkgver from
  `software/Cargo.toml` (pattern borrowed from OpenDeezer).
- Screenshots for the README are generated headless (see §10) with a neutral
  username ("Alex") — never the user's real email.
- The user is **French**; prefer replies in French.