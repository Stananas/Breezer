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
    Toggle,
    Prev,
    Next,
    Seek(f32),
    Volume(f32),
    Dock(String, String),
    ToggleRight,
    Theme(String),
    Language(String),
    OpenThemeEditor,
    SaveLayout,
    ResetLayout,
    FinishOnboarding,
    SkipOnboarding,
    TimeFormat(String),
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

enum Evt {
    Results(Vec<TrackCard>),
    Status(String),
    Auth { state: i32, username: String },
    AuthFailed(String),
    TrackChanged(TrackCard),
    Playing(bool),
    Position(f32),
    Volume(f32),
    Layout { left: String, right: String, bottom: String },
    Theme { id: String, palette: UiPalette },
    Language(String),
    PlayerEnabled(bool),
    OnboardingDone,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run() -> Result<()> {
    init_logger();

    let args = std::env::args().skip(1);
    for a in args {
        match a.as_str() {
            "--version" | "-V" => {
                println!("breezer {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--update-check" => return run_update_check(),
            "--selftest" => return run_selftest(),
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
         \x20   -V, --version   print version\n\
         \x20   --update-check  check for updates and exit\n\
         \x20   --selftest      test the audio output and exit\n\
         \x20   -h, --help      show this help\n",
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

// ---------------------------------------------------------------------------
// GUI
// ---------------------------------------------------------------------------

fn run_gui() -> Result<()> {
    let cfg = Config::load();
    let i18n = cfg.i18n();

    let window = MainWindow::new()?;

    // -- bundled translations (Slint @tr) -- must run after window creation --
    let _ = slint::select_bundled_translation(&cfg.language);

    // -- drag & drop payload bridge (DockBridge global, exported in the root) --
    {
        let bridge = window.global::<crate::DockBridge>();
        bridge.on_panel_to_transfer(slint::DataTransfer::from);
        bridge.on_transfer_to_panel(|d| {
            d.plain_text().unwrap_or_default()
        });
    }

    // -- theme & initial state ---------------------------------------------
    let mut theme = Theme::load_builtin(&cfg.theme_id).unwrap_or_else(|_| Theme::fallback());
    theme.apply_overrides(&cfg.theme_overrides);
    window.set_palette(theme.to_slint());
    window.set_theme_id(SharedString::from(cfg.theme_id.as_str()));
    window.set_ui_language(SharedString::from(cfg.language.as_str()));
    window.set_username(SharedString::from(cfg.username.clone().unwrap_or_default()));
    window.set_auth_state(if cfg.arl.is_some() { 1 } else { 0 });
    window.set_current_view(SharedString::from("home"));
    window.set_search_status(SharedString::from(i18n.t("search.prompt")));
    window.set_volume(cfg.volume);
    window.set_current_track(empty_track());
    window.set_position_label(SharedString::from("0:00"));
    window.set_duration_label(SharedString::from("0:00"));
    window.set_queue_index(-1);
    window.set_onboard_status(SharedString::from(""));
    let onboarding_seen = cfg.onboarding_done;
    window.set_onboarding_visible(!onboarding_seen);
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
    window.on_prev_requested(unary(cmd_tx.clone(), Cmd::Prev));
    window.on_next_requested(unary(cmd_tx.clone(), Cmd::Next));
    window.on_toggle_right_panel_requested(unary(cmd_tx.clone(), Cmd::ToggleRight));
    window.on_open_theme_editor_requested(unary(cmd_tx.clone(), Cmd::OpenThemeEditor));
    window.on_save_layout_requested(unary(cmd_tx.clone(), Cmd::SaveLayout));
    window.on_reset_layout_requested(unary(cmd_tx.clone(), Cmd::ResetLayout));
    window.on_onboarding_finish_requested(unary(cmd_tx.clone(), Cmd::FinishOnboarding));
    window.on_onboarding_skip_requested(unary(cmd_tx.clone(), Cmd::SkipOnboarding));
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
            let mut layout = layout;
            let plugins = PluginRegistry::with_system();
            let mut last_cards: Vec<TrackCard> = Vec::new();

            let _ = evt_tx2.send(Evt::PlayerEnabled(player.has_device())).await;

            let mut cmd_rx = cmd_rx;
            while let Some(cmd) = cmd_rx.recv().await {
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
                            }
                            Err(e) => {
                                log::warn!("search failed: {e}");
                                let msg = i18n.t_args("search.error", &[("error", &e.to_string())]);
                                let _ = evt_tx2.send(Evt::Status(msg)).await;
                            }
                        }
                    }

                    Cmd::Connect(arl) => {
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

                    Cmd::View(_view) => {}

                    Cmd::Play(id) => {
                        play_card(&last_cards, id as u64, &mut queue, &evt_tx2, &i18n, &plugins)
                            .await;
                    }
                    Cmd::PlayIndex(i) => {
                        if let Some(card) = last_cards.get(i) {
                            play_card(&last_cards, card.id as u64, &mut queue, &evt_tx2, &i18n, &plugins)
                                .await;
                        }
                    }

                    Cmd::Toggle => {
                        player.toggle();
                        let _ = evt_tx2.send(Evt::Playing(player.is_playing())).await;
                    }
                    Cmd::Prev => {
                        if let Some(card) = queue.prev() {
                            let _ = evt_tx2.send(Evt::TrackChanged(card.clone())).await;
                        }
                    }
                    Cmd::Next => {
                        if let Some(card) = queue.next() {
                            let _ = evt_tx2.send(Evt::TrackChanged(card.clone())).await;
                        }
                    }
                    Cmd::Seek(v) => {
                        player.seek(v);
                        let _ = evt_tx2.send(Evt::Position(v)).await;
                    }
                    Cmd::Volume(v) => {
                        player.set_volume(v / 100.0);
                        cfg.volume = v;
                        let _ = evt_tx2.send(Evt::Volume(v)).await;
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
                    Cmd::FinishOnboarding | Cmd::SkipOnboarding => {
                        cfg.onboarding_done = true;
                        if let Err(e) = cfg.save() {
                            log::warn!("could not save config: {e}");
                        }
                        let _ = evt_tx2.send(Evt::OnboardingDone).await;
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
            Ok::<(), Error>(())
        });
    }

    // -- update check (fire and forget) -------------------------------------
    {
        let weak = window.as_weak();
        rt.spawn(async move {
            let client = reqwest::Client::new();
            match crate::updater::check(&client).await {
                Ok(Some(info)) => {
                    let w = weak.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = w.upgrade() {
                            ui.set_search_status(SharedString::from(format!(
                                "🎉 v{} available — github.com/Breezer-App/breezer",
                                info.version
                            )));
                        }
                    });
                }
                Ok(None) => {}
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

/// Queue + start a track. v0.1: metadata & queue only — actual streaming lands
/// in v0.2 (ARL session → `Engine::play_source(decrypted, decoded source)`).
async fn play_card(
    cards: &[TrackCard],
    id: u64,
    queue: &mut PlayQueue<TrackCard>,
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
    // v0.2: fetch stream → decrypt → decode (symphonia) → Engine::play_source.
    let _ = evt_tx.send(Evt::Status(i18n.t("player.auth.required"))).await;
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
        Evt::Volume(v) => ui.set_volume(v),
        Evt::Layout { left, right, bottom } => {
            ui.set_left_panel(SharedString::from(left));
            ui.set_right_panel(SharedString::from(right));
            ui.set_bottom_panel(SharedString::from(bottom));
        }
        Evt::Theme { id, palette } => {
            ui.set_palette(palette);
            ui.set_theme_id(SharedString::from(id));
        }
        Evt::Language(code) => {
            let _ = slint::select_bundled_translation(&code);
            ui.set_ui_language(SharedString::from(code));
        }
        Evt::PlayerEnabled(enabled) => ui.set_player_enabled(enabled),
        Evt::OnboardingDone => ui.set_onboarding_visible(false),
    }
}