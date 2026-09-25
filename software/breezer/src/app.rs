//! Application bootstrap: Slint UI + tokio services wiring.
//!
//! Two async tasks around the Slint event loop:
//!  - a **services** task (Deezer API, covers, audio engine, layout manager),
//!  - a **UI bridge** task forwarding events into the event loop
//!    (`invoke_from_event_loop` + `Weak::upgrade`).
//!
//! Threading rule: `slint::Image` is not `Send`, so events crossing tasks carry
//! plain data (`TrackCard`), and images are built on the UI thread from the
//! shared cover cache.

use crate::api::ApiClient;
use crate::config::{Config, LayoutProfile, Theme};
use crate::error::{Error, Result};
use crate::i18n::I18n;
use crate::media::{empty_track, fmt_duration, Covers};
use crate::player::{Engine, PlayQueue};
use crate::plugins::PluginRegistry;
use crate::{MainWindow, Palette as UiPalette, TrackInfo};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Commands (UI → services) and events (services → UI)
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Cmd {
    Search(String),
    Connect(String),
    OpenBrowser,
    View(String),
    Play(i32),
    PlayIndex(usize),
    PlaylistSelected(String),
    Toggle,
    Prev,
    Next,
    Seek(f32),
    Volume(f32),
    Shuffle,
    Repeat,
    PlayRecent(i32),
    InstallUpdate,
    Dock(String, String),
    ToggleRight,
    Theme(String),
    ThemeOverride(String, String),
    ResetThemeOverrides,
    Language(String),
    OpenThemeEditor,
    SaveLayout,
    ResetLayout,
    FinishOnboarding,
    QuitApp,
    TimeFormat(String),
}

/// Repeat mode of the player (Deezer-style): off, whole queue, single track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    /// Cycle Off → All → One → Off (the Deezer repeat button behaviour).
    fn cycle(&self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }
    fn as_i32(&self) -> i32 {
        match self {
            RepeatMode::Off => 0,
            RepeatMode::All => 1,
            RepeatMode::One => 2,
        }
    }
}

/// Tiny xorshift32 PRNG (no external dependency) — picks a random queued
/// index different from `current` (playlist "shuffle next").
fn pick_random_other(current: usize, len: usize) -> usize {
    debug_assert!(len > 1);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0x9e3779b9);
    let mut x = (nanos ^ (current as u32).wrapping_mul(0x9e3779b9)) | 1;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    let pick = (x as usize) % (len - 1);
    if pick >= current {
        pick + 1
    } else {
        pick
    }
}

/// Send-safe track descriptor (images are reconstructed on the UI thread).
#[derive(Debug, Clone)]
struct TrackCard {
    id: i32,
    title: String,
    artist: String,
    album: String,
    cover_url: String,
    duration: f32,
}

impl TrackCard {
    fn to_ui(&self, covers: &Covers) -> TrackInfo {
        TrackInfo {
            id: self.id,
            title: self.title.clone().into(),
            artist: self.artist.clone().into(),
            album: self.album.clone().into(),
            cover: covers.image(&self.cover_url),
            duration: self.duration,
        }
    }
}

/// Send-safe playlist row (images built on the UI thread).
#[derive(Debug, Clone)]
struct PlaylistCard {
    id: String,
    title: String,
    cover_url: String,
}

enum Evt {
    Results(Vec<TrackCard>),
    Playlists(Vec<PlaylistCard>),
    Status(String),
    VolumeLabel(String),
    PositionLabel(String),
    Auth {
        state: i32,
        username: String,
    },
    AuthFailed(String),
    TrackChanged(TrackCard),
    Playing(bool),
    Position(f32),
    Volume(f32),
    Layout {
        left: String,
        right: String,
        bottom: String,
    },
    Theme {
        id: String,
        palette: UiPalette,
    },
    View {
        view: String,
        title: String,
    },
    Language(String),
    PlayerEnabled(bool),
    Shuffle(bool),
    Repeat(i32),
    Recents(Vec<TrackCard>),
    UpdateReady(String),
    OnboardingDone,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run() -> Result<()> {
    init_logger();

    // Auto-update: apply a previously downloaded release before the GUI starts
    // (it takes effect at next launch when the binary gets replaced in place).
    match crate::updater::apply_latest_ready() {
        Ok(true) => log::info!("pending update applied — will run the new version on next start"),
        Ok(false) => {}
        Err(e) => log::warn!("could not apply pending update: {e}"),
    }

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--version" | "-V" => {
                println!("breezer {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--update-check" => return run_update_check(),
            "--selftest" => return run_selftest(),
            "--playlists-test" => return run_playlists_test(),
            "--stream-test" => {
                let id = args
                    .next()
                    .ok_or_else(|| Error::Other("usage: breezer --stream-test <track_id>".into()))?
                    .parse::<u64>()
                    .map_err(|e| Error::Other(format!("invalid track id: {e}")))?;
                return run_stream_test(id);
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {}
        }
    }
    run_gui()
}

fn init_logger() {
    use env_logger::Env;
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init();
}

fn print_help() {
    println!(
        "breezer {version}\n\nUSAGE:\n    breezer [OPTIONS]\n\nOPTIONS:\n\
         \x20   -V, --version            print version\n\
         \x20   --update-check           check for updates and exit\n\
         \x20   --selftest               test the audio output and exit\n\
         \x20   --stream-test <trackid>  fetch+decrypt a stream and exit\n\
         \x20   -h, --help               show this help\n",
        version = env!("CARGO_PKG_VERSION")
    );
}

fn run_selftest() -> Result<()> {
    init_logger();
    let engine = Engine::new();
    if engine.play_self_test() {
        log::info!("audio self-test: playing 523 Hz tone for ~1s");
        std::thread::sleep(std::time::Duration::from_millis(1100));
    } else {
        log::warn!("audio self-test skipped: no output device");
    }
    Ok(())
}

fn run_update_check() -> Result<()> {
    init_logger();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let client = reqwest::Client::new();
        match crate::updater::check(&client).await? {
            Some(info) => println!("update available: v{} ({})", info.version, info.url),
            None => println!("up to date (v{})", env!("CARGO_PKG_VERSION")),
        }
        Ok(())
    })
}

/// Playlists diagnostic: list user playlists then fetch the tracks of the first
/// one (no audio playback).
fn run_playlists_test() -> Result<()> {
    init_logger();
    let cfg = Config::load();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let mut api = ApiClient::from_config(&cfg)?;
        match api
            .auth_with_arl(cfg.arl.as_deref().unwrap_or_default())
            .await
        {
            Ok(s) => {
                println!("session ok for user {}", s.user_id);
                api.set_session(cfg.arl.clone().unwrap(), &s);
            }
            Err(e) => {
                println!("no valid ARL session: {e}");
                return Ok(());
            }
        }
        match api.playlists().await {
            Ok(list) => {
                println!("{} playlists", list.len());
                if let Some(first) = list.first() {
                    println!(
                        "first: {} (id {}, {} tracks)",
                        first.title, first.id, first.count
                    );
                    match api.playlist_tracks(&first.id.to_string()).await {
                        Ok(tracks) => {
                            println!("  {} tracks fetched", tracks.len());
                            for t in tracks.iter().take(3) {
                                println!("    {} — {} (id {})", t.title, t.artist.name, t.id);
                            }
                        }
                        Err(e) => println!("  playlist_tracks failed: {e}"),
                    }
                }
            }
            Err(e) => println!("playlists failed: {e}"),
        }
        match api.last_played().await {
            Ok(Some(t)) => println!("last played: {} — {} (id {})", t.title, t.artist.name, t.id),
            Ok(None) => println!("no listening history"),
            Err(e) => println!("last played error: {e}"),
        }
        Ok(())
    })
}

/// End-to-end streaming diagnostic: getUserData → pageTrack → media.get_url →
/// download → Blowfish decrypt → verify MP3 header. No audio playback.
fn run_stream_test(track_id: u64) -> Result<()> {
    init_logger();
    let cfg = Config::load();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let mut api = ApiClient::from_config(&cfg)?;
        match api
            .auth_with_arl(cfg.arl.as_deref().unwrap_or_default())
            .await
        {
            Ok(s) => {
                println!("session ok for user {}", s.user_id);
                api.set_session(cfg.arl.clone().unwrap(), &s);
            }
            Err(e) => {
                println!("no valid ARL session: {e}");
                return Ok(());
            }
        }
        match api.stream_encrypted_mp3(track_id).await {
            Ok(enc) => {
                println!("downloaded {} encrypted bytes", enc.len());
                match crate::player::decrypt::decrypt_audio_stream(&track_id.to_string(), &enc) {
                    Ok(dec) => {
                        println!("decrypted {} bytes", dec.len());
                        let head = std::cmp::min(8, dec.len());
                        if dec.starts_with(b"ID3")
                            || (dec.first() == Some(&0xFF)
                                && (dec.get(1).copied().unwrap_or(0) & 0xE0) == 0xE0)
                        {
                            println!("✓ MP3 header OK: {:02x?}", &dec[..head]);
                        } else {
                            println!("unexpected stream header: {:02x?}", &dec[..head]);
                        }
                    }
                    Err(e) => println!("decryption failed: {e}"),
                }
            }
            Err(e) => println!("streaming fetch failed: {e}"),
        }
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// GUI
// ---------------------------------------------------------------------------

fn run_gui() -> Result<()> {
    let cfg = Config::load();
    let i18n = cfg.i18n();
    // Snapshot for the fire-and-forget update check (i18n is moved into the
    // services task below).
    let update_i18n = i18n.clone();

    let window = MainWindow::new()?;

    // -- bundled translations (Slint @tr) -- must run after window creation --
    let _ = slint::select_bundled_translation(&cfg.language);

    // -- drag & drop payload bridge (DockBridge global, exported in the root) --
    {
        let bridge = window.global::<crate::DockBridge>();
        bridge.on_panel_to_transfer(slint::DataTransfer::from);
        bridge.on_transfer_to_panel(|d| d.plain_text().unwrap_or_default());
    }

    // -- theme & initial state ---------------------------------------------
    let mut theme = Theme::load_builtin(&cfg.theme_id).unwrap_or_else(|_| Theme::fallback());
    theme.apply_overrides(&cfg.theme_overrides);
    window.set_palette(theme.to_slint());
    window.set_theme_id(SharedString::from(cfg.theme_id.as_str()));
    window.set_ui_language(SharedString::from(cfg.language.as_str()));
    window.set_username(SharedString::from(cfg.username.clone().unwrap_or_default()));
    let already_connected = cfg.arl.is_some();
    window.set_auth_state(if already_connected { 2 } else { 0 });
    window.set_current_view(SharedString::from("home"));
    window.set_view_title(SharedString::from(i18n.t("nav.home")));
    window.set_version_status(SharedString::from(format!(
        "Breezer v{}",
        env!("CARGO_PKG_VERSION")
    )));
    window.set_search_status(SharedString::from(i18n.t("search.prompt")));
    window.set_volume(cfg.volume);
    window.set_volume_label(SharedString::from(format!(
        "{} %",
        cfg.volume.round() as u32
    )));
    window.set_current_track(empty_track());
    window.set_position_label(SharedString::from("0:00"));
    window.set_duration_label(SharedString::from("0:00"));
    window.set_queue_index(-1);
    window.set_shuffle_on(false);
    window.set_repeat_mode(0);
    window.set_update_ready(SharedString::from(""));
    window.set_onboard_status(SharedString::from(""));
    let onboarding_seen = cfg.onboarding_done;
    window.set_onboarding_visible(!onboarding_seen && !already_connected);
    window.set_time_format(SharedString::from(cfg.time_format.as_str()));

    let layout = LayoutProfile::load();
    window.set_left_panel(SharedString::from(layout.left.as_str()));
    window.set_right_panel(SharedString::from(layout.right.as_str()));
    window.set_bottom_panel(SharedString::from(layout.bottom.as_str()));

    let rt = tokio::runtime::Runtime::new()?;
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>(128);
    let (evt_tx, evt_rx) = mpsc::channel::<Evt>(128);

    // -- UI → services callbacks -------------------------------------------
    use slint::SharedString as Str;
    window.on_search_requested({
        let tx = cmd_tx.clone();
        move |s: Str| {
            let _ = tx.try_send(Cmd::Search(s.to_string()));
        }
    });
    window.on_connect_requested({
        let tx = cmd_tx.clone();
        move |s: Str| {
            let _ = tx.try_send(Cmd::Connect(s.to_string()));
        }
    });
    window.on_view_requested({
        let tx = cmd_tx.clone();
        move |s: Str| {
            let _ = tx.try_send(Cmd::View(s.to_string()));
        }
    });
    window.on_theme_changed({
        let tx = cmd_tx.clone();
        move |s: Str| {
            let _ = tx.try_send(Cmd::Theme(s.to_string()));
        }
    });
    window.on_theme_override_requested({
        let tx = cmd_tx.clone();
        move |t: Str, v: Str| {
            let _ = tx.try_send(Cmd::ThemeOverride(t.to_string(), v.to_string()));
        }
    });
    window.on_reset_theme_overrides_requested({
        let tx = cmd_tx.clone();
        move || {
            let _ = tx.try_send(Cmd::ResetThemeOverrides);
        }
    });
    window.on_language_changed({
        let tx = cmd_tx.clone();
        move |s: Str| {
            let _ = tx.try_send(Cmd::Language(s.to_string()));
        }
    });
    window.on_dock_requested({
        let tx = cmd_tx.clone();
        move |p: Str, t: Str| {
            let _ = tx.try_send(Cmd::Dock(p.to_string(), t.to_string()));
        }
    });
    window.on_play_requested({
        let tx = cmd_tx.clone();
        move |id: i32| {
            let _ = tx.try_send(Cmd::Play(id));
        }
    });
    window.on_play_index_requested({
        let tx = cmd_tx.clone();
        move |i: i32| {
            let _ = tx.try_send(Cmd::PlayIndex(i.max(0) as usize));
        }
    });
    window.on_play_recent_requested({
        let tx = cmd_tx.clone();
        move |id: i32| {
            let _ = tx.try_send(Cmd::PlayRecent(id));
        }
    });
    window.on_playlist_selected({
        let tx = cmd_tx.clone();
        move |p: Str| {
            let _ = tx.try_send(Cmd::PlaylistSelected(p.to_string()));
        }
    });
    window.on_back_requested({
        let tx = cmd_tx.clone();
        move || {
            let _ = tx.try_send(Cmd::View("playlists".into()));
        }
    });
    window.on_seek_requested({
        let tx = cmd_tx.clone();
        move |v: f32| {
            let _ = tx.try_send(Cmd::Seek(v));
        }
    });
    window.on_volume_requested({
        let tx = cmd_tx.clone();
        move |v: f32| {
            let _ = tx.try_send(Cmd::Volume(v));
        }
    });

    let unary = |tx: mpsc::Sender<Cmd>, c: Cmd| {
        let tx = tx.clone();
        move || {
            let _ = tx.try_send(c.clone());
        }
    };
    window.on_open_browser_requested(unary(cmd_tx.clone(), Cmd::OpenBrowser));
    window.on_toggle_requested(unary(cmd_tx.clone(), Cmd::Toggle));
    window.on_install_update_requested(unary(cmd_tx.clone(), Cmd::InstallUpdate));
    window.on_prev_requested(unary(cmd_tx.clone(), Cmd::Prev));
    window.on_next_requested(unary(cmd_tx.clone(), Cmd::Next));
    window.on_shuffle_requested(unary(cmd_tx.clone(), Cmd::Shuffle));
    window.on_repeat_requested(unary(cmd_tx.clone(), Cmd::Repeat));
    window.on_toggle_right_panel_requested(unary(cmd_tx.clone(), Cmd::ToggleRight));
    window.on_open_theme_editor_requested(unary(cmd_tx.clone(), Cmd::OpenThemeEditor));
    window.on_save_layout_requested(unary(cmd_tx.clone(), Cmd::SaveLayout));
    window.on_reset_layout_requested(unary(cmd_tx.clone(), Cmd::ResetLayout));
    window.on_onboarding_finish_requested(unary(cmd_tx.clone(), Cmd::FinishOnboarding));
    window.on_onboarding_quit_requested(unary(cmd_tx.clone(), Cmd::QuitApp));
    window.on_time_format_requested({
        let tx = cmd_tx.clone();
        move |s: Str| {
            let _ = tx.try_send(Cmd::TimeFormat(s.to_string()));
        }
    });

    // -- UI bridge: services → events → event loop -------------------------
    let covers = Covers::new(reqwest::Client::new());
    {
        let weak = window.as_weak();
        let covers = covers.clone();
        rt.spawn(async move {
            let mut evt_rx = evt_rx;
            while let Some(evt) = evt_rx.recv().await {
                let w = weak.clone();
                let covers = covers.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = w.upgrade() {
                        apply_event(&ui, evt, &covers);
                    }
                });
            }
        });
    }

    // -- services task ------------------------------------------------------
    {
        let evt_tx2 = evt_tx.clone();
        let cmd_rx = cmd_rx;
        let covers = covers.clone();
        rt.spawn(async move {
            let mut cfg = cfg;
            let mut i18n = i18n;
            let mut api = ApiClient::from_config(&cfg)?;
            let mut player = Engine::new();
            let mut queue: PlayQueue<TrackCard> = PlayQueue::new();
            let mut playlists_cache: Vec<PlaylistCard> = Vec::new();

            // session_id + license_token are session-scoped (not persisted):
            // re-validate the stored ARL at startup to refresh them.
            if let Some(arl) = cfg.arl.clone() {
                match api.auth_with_arl(&arl).await {
                    Ok(s) => {
                        log::info!("session refreshed for {}", s.username);
                        api.set_session(arl, &s);
                        // Already connected → skip the wizard and reflect the
                        // logged-in state immediately.
                        if !cfg.onboarding_done {
                            cfg.onboarding_done = true;
                            let _ = cfg.save();
                        }
                        let _ = evt_tx2.send(Evt::OnboardingDone).await;
                        let _ = evt_tx2
                            .send(Evt::Auth { state: 2, username: s.username })
                            .await;

                        // Cross-device resume: load the last listened track into the player
                        // (NOT started automatically — press Play to listen).
                        match api.last_played().await {
                            Ok(Some(t)) => {
                                let duration = if t.duration > 0 {
                                    t.duration as f32
                                } else {
                                    api.track_duration(t.id).await.unwrap_or(0) as f32
                                };
                                let card = TrackCard {
                                    id: t.id as i32,
                                    title: t.title,
                                    artist: t.artist.name,
                                    album: String::new(),
                                    cover_url: t.album.cover_medium,
                                    duration,
                                };
                                let cover_url = card.cover_url.clone();
                                covers.ensure(&[cover_url]).await;
                                let _ = evt_tx2.send(Evt::TrackChanged(card.clone())).await;
                                queue.set_tracks(vec![card.clone()], 0);
                                log::info!("loaded last played (paused): {}", card.title);
                            }
                            Ok(None) => {}
                            Err(e) => log::debug!("last played unavailable: {e}"),
                        }

                        // Pre-load the playlists list for the sidebar.
                        if playlists_cache.is_empty() {
                            match api.playlists().await {
                                Ok(list) => {
                                    let urls: Vec<String> = list
                                        .iter()
                                        .map(|p| playlist_cover_url(&p.picture_hash))
                                        .collect();
                                    covers.ensure(&urls).await;
                                    playlists_cache = list
                                        .into_iter()
                                        .map(|p| PlaylistCard {
                                            id: p.id.to_string(),
                                            title: p.title,
                                            cover_url: playlist_cover_url(&p.picture_hash),
                                        })
                                        .collect();
                                    let _ = evt_tx2
                                        .send(Evt::Playlists(playlists_cache.clone()))
                                        .await;
                                }
                                Err(e) => log::debug!("playlists prefetch failed: {e}"),
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("stored ARL no longer valid: {e}");
                        let _ = evt_tx2
                            .send(Evt::Auth { state: 1, username: String::new() })
                            .await;
                    }
                }
            }

            let mut layout = layout;
            let plugins = PluginRegistry::with_system();
            let mut last_cards: Vec<TrackCard> = Vec::new();
            let mut recent_cards: Vec<TrackCard> = Vec::new();
            let mut shuffle_on = false;
            let mut repeat_mode = RepeatMode::Off;

            // Seed the Home page with Deezer's trending tracks (public chart).
            match api.chart(24).await {
                Ok(tracks) => {
                    let urls: Vec<String> = tracks
                        .iter()
                        .map(|t| t.album.cover_medium.clone())
                        .collect();
                    covers.ensure(&urls).await;
                    let cards: Vec<TrackCard> = tracks
                        .into_iter()
                        .map(|t| TrackCard {
                            id: t.id as i32,
                            title: t.title,
                            artist: t.artist.name,
                            album: t.album.title,
                            cover_url: t.album.cover_medium,
                            duration: t.duration as f32,
                        })
                        .collect();
                    last_cards = cards.clone();
                    let _ = evt_tx2.send(Evt::Results(cards)).await;
                    let _ = evt_tx2.send(Evt::Status(String::new())).await;
                }
                Err(e) => log::debug!("chart prefetch failed: {e}"),
            }

            // Home page "Recently played" shelf: the full listening history
            // (newest first), so the first shelf is not limited to one track.
            match api.recent_played(12).await {
                Ok(recent) => {
                    let urls: Vec<String> = recent
                        .iter()
                        .map(|t| t.album.cover_medium.clone())
                        .collect();
                    covers.ensure(&urls).await;
                    recent_cards = recent
                        .into_iter()
                        .map(|t| TrackCard {
                            id: t.id as i32,
                            title: t.title,
                            artist: t.artist.name,
                            album: t.album.title,
                            cover_url: t.album.cover_medium,
                            duration: t.duration as f32,
                        })
                        .collect();
                    if !recent_cards.is_empty() {
                        let _ = evt_tx2.send(Evt::Recents(recent_cards.clone())).await;
                    }
                }
                Err(e) => log::debug!("recent history prefetch failed: {e}"),
            }

            let _ = evt_tx2.send(Evt::PlayerEnabled(player.has_device())).await;

            let mut cmd_rx = cmd_rx;
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));
            loop {
                tokio::select! {
                    Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    Cmd::Search(q) => {
                        covers.clear();
                        let _ = evt_tx2.send(Evt::Status(i18n.t("search.loading"))).await;
                        match api.search_tracks(&q, 24).await {
                            Ok(tracks) => {
                                let urls: Vec<String> = tracks
                                    .iter()
                                    .map(|t| t.album.cover_medium.clone())
                                    .collect();
                                covers.ensure(&urls).await;

                                let cards: Vec<TrackCard> = tracks
                                    .into_iter()
                                    .map(|t| TrackCard {
                                        id: t.id as i32,
                                        title: t.title,
                                        artist: t.artist.name,
                                        album: t.album.title,
                                        cover_url: t.album.cover_medium,
                                        duration: t.duration as f32,
                                    })
                                    .collect();
                                last_cards = cards.clone();
                                let _ = evt_tx2.send(Evt::Results(cards)).await;
                                let msg = if last_cards.is_empty() {
                                    i18n.t("search.empty")
                                } else {
                                    i18n.t_args(
                                        "search.results",
                                        &[("count", &last_cards.len().to_string()), ("query", &q)],
                                    )
                                };
                                let _ = evt_tx2.send(Evt::Status(msg)).await;
                                // Searching opens the results shelf (Explorer),
                                // like Deezer — Home keeps its curated shelves.
                                if !last_cards.is_empty() {
                                    let _ = evt_tx2
                                        .send(Evt::View {
                                            view: "explore".into(),
                                            title: i18n.t("nav.explore"),
                                        })
                                        .await;
                                }
                            }
                            Err(e) => {
                                log::warn!("search failed: {e}");
                                let msg = i18n.t_args("search.error", &[("error", &e.to_string())]);
                                let _ = evt_tx2.send(Evt::Status(msg)).await;
                            }
                        }
                    }

                    Cmd::Connect(arl) => {
                        if arl.trim().is_empty() {
                            // No ARL pasted: guide the user instead of failing.
                            if let Err(e) = webbrowser::open("https://www.deezer.com/login") {
                                log::warn!("could not open browser: {e}");
                            }
                            let _ = evt_tx2
                                .send(Evt::Status(i18n.t("auth.paste.arl")))
                                .await;
                            continue;
                        }
                        let _ = evt_tx2.send(Evt::Status(i18n.t("auth.connecting"))).await;
                        match api.auth_with_arl(&arl).await {
                            Ok(session) => {
                                api.set_session(arl.clone(), &session);
                                cfg.arl = Some(arl);
                                cfg.api_token = Some(session.api_token.clone());
                                cfg.username = Some(session.username.clone());
                                if let Err(e) = cfg.save() {
                                    log::warn!("could not save config: {e}");
                                }
                                plugins.notify_auth(true, &session.username);
                                let _ = evt_tx2
                                    .send(Evt::Auth {
                                        state: 2,
                                        username: session.username.clone(),
                                    })
                                    .await;
                                let msg =
                                    i18n.t_args("auth.success", &[("username", &session.username)]);
                                let _ = evt_tx2.send(Evt::Status(msg)).await;
                                // Login is mandatory: successful connection also
                                // completes the first-launch onboarding.
                                if !cfg.onboarding_done {
                                    cfg.onboarding_done = true;
                                    if let Err(e) = cfg.save() {
                                        log::warn!("could not save config: {e}");
                                    }
                                    let _ = evt_tx2.send(Evt::OnboardingDone).await;
                                }
                            }
                            Err(e) => {
                                log::warn!("auth failed: {e}");
                                let _ = evt_tx2.send(Evt::AuthFailed(i18n.t("auth.failed"))).await;
                            }
                        }
                    }

                    Cmd::OpenBrowser => {
                        if let Err(e) = webbrowser::open("https://www.deezer.com/login") {
                            log::warn!("could not open browser: {e}");
                        }
                    }

                    Cmd::View(v) => {
                        let title = match v.as_str() {
                            "home" => i18n.t("nav.home"),
                            "explore" => i18n.t("nav.explore"),
                            "favorites" => i18n.t("nav.favorites"),
                            "playlists" => i18n.t("nav.playlists"),
                            "settings" => i18n.t("nav.settings"),
                            _ => v.clone(),
                        };
                        let _ = evt_tx2.send(Evt::View { view: v.clone(), title }).await;

                        // Fetch the user's playlists when entering the view
                        // (cached afterwards — the "back" button reuses it).
                        if v == "playlists" && playlists_cache.is_empty() && api.is_authenticated() {
                            match api.playlists().await {
                                Ok(list) => {
                                    let urls: Vec<String> =
                                        list.iter().map(|p| playlist_cover_url(&p.picture_hash)).collect();
                                    covers.ensure(&urls).await;
                                    playlists_cache = list
                                        .into_iter()
                                        .map(|p| PlaylistCard {
                                            id: p.id.to_string(),
                                            title: p.title,
                                            cover_url: playlist_cover_url(&p.picture_hash),
                                        })
                                        .collect();
                                    let _ = evt_tx2.send(Evt::Playlists(playlists_cache.clone())).await;
                                }
                                Err(e) => {
                                    log::warn!("playlists fetch failed: {e}");
                                    let _ = evt_tx2
                                        .send(Evt::Status(
                                            i18n.t_args("playlists.error", &[("error", &e.to_string())]),
                                        )).await;
                                }
                            }
                        }
                    }

                    Cmd::PlaylistSelected(pid) => {
                        let title = playlists_cache
                            .iter()
                            .find(|p| p.id == pid)
                            .map(|p| p.title.clone())
                            .unwrap_or_else(|| i18n.t("nav.playlists"));
                        match api.playlist_tracks(&pid).await {
                            Ok(tracks) => {
                                let urls: Vec<String> = tracks
                                    .iter()
                                    .map(|t| t.album.cover_medium.clone())
                                    .collect();
                                covers.ensure(&urls).await;
                                let cards: Vec<TrackCard> = tracks
                                    .into_iter()
                                    .map(|t| TrackCard {
                                        id: t.id as i32,
                                        title: t.title,
                                        artist: t.artist.name,
                                        album: t.album.title,
                                        cover_url: t.album.cover_medium,
                                        duration: t.duration as f32,
                                    })
                                    .collect();
                                let count = cards.len();
                                last_cards = cards.clone();
                                let _ = evt_tx2
                                    .send(Evt::View { view: "playlist-detail".into(), title })
                                    .await;
                                let _ = evt_tx2.send(Evt::Results(cards)).await;
                                let _ = evt_tx2
                                    .send(Evt::Status(
                                        i18n.t_args("playlist.tracks", &[("count", &count.to_string())]),
                                    ))
                                    .await;
                            }
                            Err(e) => {
                                log::warn!("playlist tracks failed: {e}");
                                let _ = evt_tx2
                                    .send(Evt::Status(
                                        i18n.t_args("playlist.tracks.error", &[("error", &e.to_string())]),
                                    ))
                                    .await;
                            }
                        }
                    }

                    Cmd::Play(id) => {
                        play_card(
                            &last_cards,
                            id as u64,
                            &mut player,
                            &api,
                            &mut queue,
                            &mut cfg,
                            &evt_tx2,
                            &i18n,
                            &plugins,
                        )
                        .await;
                    }
                    Cmd::PlayIndex(i) => {
                        if let Some(card) = last_cards.get(i) {
                            play_card(
                                &last_cards,
                                card.id as u64,
                                &mut player,
                                &api,
                                &mut queue,
                                &mut cfg,
                                &evt_tx2,
                                &i18n,
                                &plugins,
                            )
                            .await;
                        }
                    }
                    Cmd::PlayRecent(id) => {
                        if recent_cards.iter().any(|c| c.id == id) {
                            play_card(
                                &recent_cards,
                                id as u64,
                                &mut player,
                                &api,
                                &mut queue,
                                &mut cfg,
                                &evt_tx2,
                                &i18n,
                                &plugins,
                            )
                            .await;
                        }
                    }

                    Cmd::Toggle => {
                        if !player.has_loaded() {
                            // Nothing decoded yet — (re)start the current track
                            // (e.g. the "last played" loaded at boot).
                            if let Some(card) = queue.current().cloned() {
                                start_streaming(card, &mut player, &api, &evt_tx2, &i18n).await;
                            }
                        } else {
                            player.toggle();
                        }
                        let _ = evt_tx2.send(Evt::Playing(player.is_playing())).await;
                    }
                    Cmd::Prev => {
                        if let Some(card) = queue.prev().cloned() {
                            let _ = evt_tx2.send(Evt::TrackChanged(card.clone())).await;
                            start_streaming(card, &mut player, &api, &evt_tx2, &i18n).await;
                        }
                    }
                    Cmd::Next => {
                        advance_playback(
                            &mut queue,
                            shuffle_on,
                            repeat_mode,
                            true,
                            &mut player,
                            &api,
                            &evt_tx2,
                            &i18n,
                        )
                        .await;
                    }
                    Cmd::Shuffle => {
                        shuffle_on = !shuffle_on;
                        let _ = evt_tx2.send(Evt::Shuffle(shuffle_on)).await;
                    }
                    Cmd::Repeat => {
                        repeat_mode = repeat_mode.cycle();
                        let _ = evt_tx2.send(Evt::Repeat(repeat_mode.as_i32())).await;
                    }
                    Cmd::Seek(v) => {
                        player.seek(v);
                        let _ = evt_tx2.send(Evt::Position(v)).await;
                    }
                    Cmd::Volume(v) => {
                        player.set_volume(v / 100.0);
                        cfg.volume = v;
                        let _ = evt_tx2.send(Evt::Volume(v)).await;
                        let _ = evt_tx2
                            .send(Evt::VolumeLabel(format!("{} %", v.round() as u32)))
                            .await;
                        if let Err(e) = cfg.save() {
                            log::warn!("could not save config: {e}");
                        }
                    }

                    Cmd::Dock(panel, target) => {
                        layout.set_panel(&panel, &target);
                        persist_layout(&layout, &plugins);
                        let _ = evt_tx2.send(layout_event(&layout)).await;
                    }
                    Cmd::ToggleRight => {
                        layout.toggle_queue();
                        persist_layout(&layout, &plugins);
                        let _ = evt_tx2.send(layout_event(&layout)).await;
                    }
                    Cmd::SaveLayout => {
                        if let Err(e) = layout.save() {
                            log::warn!("could not save layout: {e}");
                        }
                        let _ = evt_tx2.send(Evt::Status(i18n.t("layout.saved"))).await;
                    }
                    Cmd::ResetLayout => {
                        layout.reset();
                        persist_layout(&layout, &plugins);
                        let _ = evt_tx2.send(layout_event(&layout)).await;
                    }

                    Cmd::Theme(id) => {
                        cfg.theme_id = id.clone();
                        if let Err(e) = cfg.save() {
                            log::warn!("could not save config: {e}");
                        }
                        let mut theme =
                            Theme::load_builtin(&id).unwrap_or_else(|_| Theme::fallback());
                        theme.apply_overrides(&cfg.theme_overrides);
                        plugins.notify_theme(&id);
                        let _ = evt_tx2
                            .send(Evt::Theme {
                                id,
                                palette: theme.to_slint(),
                            })
                            .await;
                    }
                    Cmd::ThemeOverride(token, value) => {
                        let known = matches!(
                            token.as_str(),
                            "bg" | "surface" | "surface2"
                                | "primary" | "on-primary" | "accent"
                                | "text" | "text-secondary" | "error"
                                | "radius" | "spacing"
                        );
                        if !known {
                            continue;
                        }
                        if value.trim().is_empty() {
                            cfg.theme_overrides.remove(&token);
                        } else {
                            cfg.theme_overrides.insert(token, value);
                        }
                        if let Err(e) = cfg.save() {
                            log::warn!("could not save config: {e}");
                        }
                        let mut theme = Theme::load_builtin(&cfg.theme_id)
                            .unwrap_or_else(|_| Theme::fallback());
                        theme.apply_overrides(&cfg.theme_overrides);
                        let _ = evt_tx2
                            .send(Evt::Theme {
                                id: cfg.theme_id.clone(),
                                palette: theme.to_slint(),
                            })
                            .await;
                    }
                    Cmd::ResetThemeOverrides => {
                        cfg.theme_overrides.clear();
                        if let Err(e) = cfg.save() {
                            log::warn!("could not save config: {e}");
                        }
                        let _ = evt_tx2
                            .send(Evt::Theme {
                                id: cfg.theme_id.clone(),
                                palette: Theme::load_builtin(&cfg.theme_id)
                                    .unwrap_or_else(|_| Theme::fallback())
                                    .to_slint(),
                            })
                            .await;
                    }
                    Cmd::Language(code) => {
                        cfg.language = code.clone();
                        if let Err(e) = cfg.save() {
                            log::warn!("could not save config: {e}");
                        }
                        i18n = I18n::from_code(&code);
                        let msg =
                            i18n.t_args("language.applied", &[("language", &code.to_uppercase())]);
                        let _ = evt_tx2.send(Evt::Language(code)).await;
                        let _ = evt_tx2.send(Evt::Status(msg)).await;
                    }
                    Cmd::OpenThemeEditor => {
                        // v0.2: in-app color editor dialog.
                        let _ = evt_tx2.send(Evt::Status(i18n.t("theme.applied"))).await;
                    }
                    Cmd::FinishOnboarding => {
                        // Deezer login is mandatory: refuse until connected.
                        if cfg.arl.is_some() {
                            cfg.onboarding_done = true;
                            if let Err(e) = cfg.save() {
                                log::warn!("could not save config: {e}");
                            }
                            let _ = evt_tx2.send(Evt::OnboardingDone).await;
                        } else {
                            if let Err(e) = webbrowser::open("https://www.deezer.com/login") {
                                log::warn!("could not open browser: {e}");
                            }
                            let _ = evt_tx2
                                .send(Evt::Status(i18n.t("auth.connect.first")))
                                .await;
                        }
                    }
                    Cmd::QuitApp => {
                        let _ = slint::quit_event_loop();
                        break;
                    }
                    Cmd::InstallUpdate => {
                        match crate::updater::apply_latest_ready() {
                            Ok(true) => {
                                let msg = i18n.t("update.ready");
                                let _ = evt_tx2.send(Evt::Status(msg)).await;
                                let exe = std::env::current_exe().unwrap_or_default();
                                let _ = crate::updater::restart(&exe);
                                let _ = slint::quit_event_loop();
                                break;
                            }
                            Ok(false) => {
                                let _ = evt_tx2.send(Evt::Status(i18n.t("update.none"))).await;
                            }
                            Err(e) => {
                                let msg =
                                    i18n.t_args("update.install.failed", &[("error", &e.to_string())]);
                                let _ = evt_tx2.send(Evt::Status(msg)).await;
                            }
                        }
                    }
                    Cmd::TimeFormat(fmt) => {
                        if fmt == "24h" || fmt == "12h" {
                            cfg.time_format = fmt;
                            if let Err(e) = cfg.save() {
                                log::warn!("could not save config: {e}");
                            }
                        }
                    }
                }
                    }
                    _ = tick.tick() => {
                        // Progress reporter: keep the slider + elapsed label in
                        // sync while playing, and auto-advance on track end.
                        if player.is_playing() {
                            if let Some(pos) = player.position_secs() {
                                let _ = evt_tx2.send(Evt::Position(pos)).await;
                                let _ = evt_tx2
                                    .send(Evt::PositionLabel(fmt_duration(pos)))
                                    .await;
                            }
                            if player.ended() {
                                player.stop();
                                advance_playback(
                                    &mut queue,
                                    shuffle_on,
                                    repeat_mode,
                                    false,
                                    &mut player,
                                    &api,
                                    &evt_tx2,
                                    &i18n,
                                )
                                .await;
                            }
                        }
                    }
                }
            }
            Ok::<(), Error>(())
        });
    }

    // -- auto-update (fire and forget): check + download in the background. -----
    // The new binary is downloaded automatically; the UI then offers to
    // "Install & restart" (no GitHub round-trip for the user).
    {
        let weak = window.as_weak();
        let evt3 = evt_tx.clone();
        rt.spawn(async move {
            let client = reqwest::Client::new();
            match crate::updater::check(&client).await {
                Ok(Some(info)) => {
                    let v = format!("v{}", info.version);
                    match crate::updater::prepare_latest_update(&client, &info).await {
                        Ok(Some(version)) => {
                            log::info!("update {} ready to install", version);
                            let _ = evt3.send(Evt::UpdateReady(version.to_string())).await;
                        }
                        Ok(None) => {
                            // No compatible build for this platform — point to GitHub.
                            let msg = update_i18n.t_args("update.available", &[("version", &v)]);
                            let w = weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = w.upgrade() {
                                    ui.set_version_status(SharedString::from(msg));
                                }
                            });
                        }
                        Err(e) => {
                            log::warn!("update download failed: {e}");
                            let msg = update_i18n
                                .t_args("update.download.failed", &[("error", &e.to_string())]);
                            let _ = evt3.send(Evt::Status(msg)).await;
                        }
                    }
                }
                Ok(None) => {
                    let msg = update_i18n.t("update.none");
                    let w = weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = w.upgrade() {
                            ui.set_version_status(SharedString::from(msg));
                        }
                    });
                }
                Err(e) => log::debug!("update check skipped: {e}"),
            }
        });
    }

    window.run()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn persist_layout(layout: &LayoutProfile, plugins: &PluginRegistry) {
    if let Err(e) = layout.save() {
        log::warn!("could not save layout: {e}");
    }
    plugins.notify_layout(&layout.to_json());
}

fn layout_event(layout: &LayoutProfile) -> Evt {
    Evt::Layout {
        left: layout.left.clone(),
        right: layout.right.clone(),
        bottom: layout.bottom.clone(),
    }
}

/// Cover URL for a Deezer playlist/album picture hash.
fn playlist_cover_url(hash: &str) -> String {
    if hash.is_empty() {
        String::new()
    } else {
        format!("https://e-cdns-images.dzcdn.net/images/cover/{hash}/250x250-000000-80-0-0.jpg")
    }
}

/// Queue + start a track.
#[allow(clippy::too_many_arguments)]
async fn play_card(
    cards: &[TrackCard],
    id: u64,
    player: &mut Engine,
    api: &ApiClient,
    queue: &mut PlayQueue<TrackCard>,
    cfg: &mut Config,
    evt_tx: &mpsc::Sender<Evt>,
    i18n: &I18n,
    plugins: &PluginRegistry,
) {
    let Some(pos) = cards.iter().position(|c| c.id as u64 == id) else {
        return;
    };
    let card = cards[pos].clone();

    let _ = evt_tx.send(Evt::TrackChanged(card.clone())).await;
    queue.set_tracks(cards.to_vec(), pos);
    plugins.notify_track(&breezer_plugin_api::TrackMeta {
        id: card.id as i64,
        title: card.title.clone(),
        artist: card.artist.clone(),
        album: card.album.clone(),
    });

    if cfg.arl.is_none() {
        let _ = evt_tx
            .send(Evt::Status(i18n.t("player.auth.required")))
            .await;
        return;
    }
    start_streaming(card, player, api, evt_tx, i18n).await;
}

/// Full Deezer streaming flow (mirroring the tui-dzr client):
/// pageTrack → media.get_url (BF_CBC_STRIPE/MP3_128) → download → Blowfish
/// decryption → rodio MP3 playback.
async fn start_streaming(
    card: TrackCard,
    player: &mut Engine,
    api: &ApiClient,
    evt_tx: &mpsc::Sender<Evt>,
    i18n: &I18n,
) {
    if !player.has_device() {
        let _ = evt_tx.send(Evt::Status(i18n.t("player.no.device"))).await;
        return;
    }
    let _ = evt_tx.send(Evt::Status(i18n.t("player.streaming"))).await;
    let encrypted = match api.stream_encrypted_mp3(card.id as u64).await {
        Ok(b) => b,
        Err(e) => {
            log::warn!("streaming fetch failed: {e}");
            let _ = evt_tx
                .send(Evt::Status(
                    i18n.t_args("player.stream.error", &[("error", &e.to_string())]),
                ))
                .await;
            return;
        }
    };
    let decrypted =
        match crate::player::decrypt::decrypt_audio_stream(&card.id.to_string(), &encrypted) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("stream decryption failed: {e}");
                let _ = evt_tx
                    .send(Evt::Status(
                        i18n.t_args("player.stream.error", &[("error", &e.to_string())]),
                    ))
                    .await;
                return;
            }
        };
    match player.play_mp3_bytes(decrypted) {
        Ok(()) => {
            let _ = evt_tx.send(Evt::Playing(true)).await;
        }
        Err(e) => {
            let _ = evt_tx
                .send(Evt::Status(
                    i18n.t_args("player.stream.error", &[("error", &e.to_string())]),
                ))
                .await;
        }
    }
}

/// Advance to the next queued track respecting shuffle / repeat preferences.
/// `ignore_one` is set when the user explicitly pressed "next" (with repeat
/// "one", manual next still moves to the following track, like Deezer).
/// Returns true if a new (or the same) track started playing.
#[allow(clippy::too_many_arguments)]
async fn advance_playback(
    queue: &mut PlayQueue<TrackCard>,
    shuffle: bool,
    repeat: RepeatMode,
    ignore_one: bool,
    player: &mut Engine,
    api: &ApiClient,
    evt_tx: &mpsc::Sender<Evt>,
    i18n: &I18n,
) -> bool {
    let len = queue.len();
    let card = if len == 0 {
        None
    } else if shuffle && len > 1 {
        let cur = queue.index().unwrap_or(0);
        queue.jump_to(pick_random_other(cur, len)).cloned()
    } else if repeat == RepeatMode::One && !ignore_one {
        queue.current().cloned() // replay the same track
    } else {
        let at_last = queue.index() == Some(len - 1);
        if at_last {
            if repeat == RepeatMode::All {
                queue.jump_to(0).cloned() // wrap around
            } else {
                None // end of queue — stay put
            }
        } else {
            queue.next().cloned()
        }
    };
    match card {
        Some(c) => {
            let _ = evt_tx.send(Evt::TrackChanged(c.clone())).await;
            start_streaming(c, player, api, evt_tx, i18n).await;
            true
        }
        None => {
            let _ = evt_tx.send(Evt::Playing(false)).await;
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Event application on the UI thread
// ---------------------------------------------------------------------------

fn apply_event(ui: &MainWindow, evt: Evt, covers: &Covers) {
    match evt {
        Evt::Results(cards) => {
            let items: Vec<TrackInfo> = cards.iter().map(|c| c.to_ui(covers)).collect();
            ui.set_search_results(ModelRc::from(std::rc::Rc::new(VecModel::from(items))));
        }
        Evt::Recents(cards) => {
            let items: Vec<TrackInfo> = cards.iter().map(|c| c.to_ui(covers)).collect();
            ui.set_recent_results(ModelRc::from(std::rc::Rc::new(VecModel::from(items))));
        }
        Evt::UpdateReady(version) => ui.set_update_ready(SharedString::from(version)),
        Evt::Playlists(cards) => {
            let items: Vec<crate::PlaylistInfo> = cards
                .iter()
                .map(|c| crate::PlaylistInfo {
                    id: c.id.clone().into(),
                    title: c.title.clone().into(),
                    count: 0,
                    cover: covers.image(&c.cover_url),
                })
                .collect();
            ui.set_playlist_results(ModelRc::from(std::rc::Rc::new(VecModel::from(items))));
        }
        Evt::Status(s) => {
            // An empty status hides the status line (only when results exist).
            if !s.is_empty() {
                ui.set_search_status(SharedString::from(s));
            }
        }
        Evt::Auth { state, username } => {
            ui.set_auth_state(state);
            ui.set_username(SharedString::from(username));
            ui.set_onboard_status(SharedString::from(""));
        }
        Evt::AuthFailed(msg) => {
            ui.set_search_status(SharedString::from(msg.clone()));
            ui.set_onboard_status(SharedString::from(msg));
        }
        Evt::TrackChanged(card) => {
            let track = card.to_ui(covers);
            let duration = track.duration.max(1.0);
            ui.set_current_track(track);
            ui.set_position(0.0);
            ui.set_progress_duration(duration);
            ui.set_position_label(SharedString::from("0:00"));
            ui.set_duration_label(SharedString::from(fmt_duration(duration)));
            ui.set_queue_index(0);
        }
        Evt::Playing(b) => ui.set_playing(b),
        Evt::Position(v) => ui.set_position(v),
        Evt::PositionLabel(s) => ui.set_position_label(SharedString::from(s)),
        Evt::Volume(v) => ui.set_volume(v),
        Evt::VolumeLabel(s) => ui.set_volume_label(SharedString::from(s)),
        Evt::Shuffle(on) => ui.set_shuffle_on(on),
        Evt::Repeat(mode) => ui.set_repeat_mode(mode),
        Evt::Layout {
            left,
            right,
            bottom,
        } => {
            ui.set_left_panel(SharedString::from(left));
            ui.set_right_panel(SharedString::from(right));
            ui.set_bottom_panel(SharedString::from(bottom));
        }
        Evt::Theme { id, palette } => {
            ui.set_palette(palette);
            ui.set_theme_id(SharedString::from(id));
        }
        Evt::View { view, title } => {
            ui.set_current_view(SharedString::from(view));
            ui.set_view_title(SharedString::from(title));
        }
        Evt::Language(code) => {
            let _ = slint::select_bundled_translation(&code);
            ui.set_ui_language(SharedString::from(code));
        }
        Evt::PlayerEnabled(enabled) => ui.set_player_enabled(enabled),
        Evt::OnboardingDone => ui.set_onboarding_visible(false),
    }
}
