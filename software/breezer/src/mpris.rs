//! MPRIS D-Bus service (`org.mpris.MediaPlayer2.breezer`).
//!
//! Exposes Breezer to desktop media widgets — Waybar media module, KDE Plasma
//! widget, GNOME shell, "dankmaterial"-style hotbars, etc. — and forwards
//! their Play / Pause / Next / Previous / Seek / Volume back into the app.
//!
//! Linux-only (D-Bus). The services loop pushes lightweight [`MprisMsg`]
//! updates; the long-running task owns the `LocalServer` (zbus) and emits the
//! MPRIS signals.

use std::sync::{
    atomic::{AtomicI64, AtomicU8, Ordering},
    Arc, Mutex,
};

use mpris_server::{
    zbus::{fdo, Result},
    LocalPlayerInterface, LocalRootInterface, LocalServer, LoopStatus, Metadata, PlaybackRate,
    PlaybackStatus, Property, Signal, Time, TrackId, Volume,
};
use tokio::sync::mpsc;

use crate::app::Cmd;

/// Flattened playback status (D-Bus-independent).
pub mod st {
    pub const STOPPED: u8 = 0;
    pub const PLAYING: u8 = 1;
    pub const PAUSED: u8 = 2;
}

/// Updates pushed from the services loop to the MPRIS task.
#[derive(Debug, Clone)]
pub enum MprisMsg {
    Status(u8),
    Track {
        title: String,
        artist: String,
        album: String,
        art_url: String,
        duration_secs: i64,
        id: i64,
    },
    /// Position in seconds.
    Position(i64),
    /// Volume 0.0..1.0.
    Volume(f64),
}

/// Thread-shared state read by the D-Bus getters.
struct Shared {
    status: AtomicU8,
    track_id: AtomicI64,
    title: Mutex<String>,
    artist: Mutex<String>,
    album: Mutex<String>,
    art_url: Mutex<String>,
    duration_us: AtomicI64,
    position_us: AtomicI64,
    volume: Mutex<f64>,
}

/// Interface implementation owned by the `LocalServer`.
struct BreezerMpris {
    shared: Arc<Shared>,
    cmd: mpsc::Sender<Cmd>,
}

/// Spawn the MPRIS service; returns the sender used to push updates.
///
/// The `LocalServer` future is not `Send`, so it runs on its own single-thread
/// tokio runtime inside a dedicated thread.
pub fn start(cmd_tx: mpsc::Sender<Cmd>) -> mpsc::Sender<MprisMsg> {
    let (tx, rx) = mpsc::channel(32);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("mpris runtime");
        rt.block_on(run(cmd_tx, rx));
    });
    tx
}

async fn run(cmd_tx: mpsc::Sender<Cmd>, mut rx: mpsc::Receiver<MprisMsg>) {
    let shared = Arc::new(Shared {
        status: AtomicU8::new(st::STOPPED),
        track_id: AtomicI64::new(-1),
        title: Mutex::new(String::new()),
        artist: Mutex::new(String::new()),
        album: Mutex::new(String::new()),
        art_url: Mutex::new(String::new()),
        duration_us: AtomicI64::new(0),
        position_us: AtomicI64::new(0),
        volume: Mutex::new(0.7),
    });

    let server = match LocalServer::new(
        "breezer",
        BreezerMpris {
            shared: shared.clone(),
            cmd: cmd_tx,
        },
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            log::warn!("MPRIS disabled (no session bus?): {e}");
            return;
        }
    };
    log::info!("MPRIS bus: {}", server.bus_name());
    let runner = server.run();
    tokio::pin!(runner);

    loop {
        tokio::select! {
            _ = &mut runner => {
                // The D-Bus server stopped by itself.
                log::warn!("MPRIS server task ended");
                break;
            }
            msg = rx.recv() => {
                let Some(msg) = msg else { break };
                match msg {
            MprisMsg::Status(s) => {
                let status = match s {
                    st::PLAYING => PlaybackStatus::Playing,
                    st::PAUSED => PlaybackStatus::Paused,
                    _ => PlaybackStatus::Stopped,
                };
                shared.status.store(s, Ordering::Relaxed);
                let _ = server
                    .properties_changed([Property::PlaybackStatus(status)])
                    .await;
            }
            MprisMsg::Track {
                title,
                artist,
                album,
                art_url,
                duration_secs,
                id,
            } => {
                *shared.title.lock().unwrap() = title.clone();
                *shared.artist.lock().unwrap() = artist.clone();
                *shared.album.lock().unwrap() = album.clone();
                *shared.art_url.lock().unwrap() = art_url.clone();
                shared.track_id.store(id, Ordering::Relaxed);
                shared
                    .duration_us
                    .store(duration_secs.max(0).saturating_mul(1_000_000), Ordering::Relaxed);
                shared.position_us.store(0, Ordering::Relaxed);

                let mut meta = Metadata::new();
                meta.set_trackid(Some(TrackId::NO_TRACK));
                meta.set_title(Some(title));
                meta.set_artist(Some(artist.split(',').map(|s| s.trim().to_string())));
                if !album.is_empty() {
                    meta.set_album(Some(album));
                }
                if !art_url.is_empty() {
                    meta.set_art_url(Some(art_url));
                }
                if duration_secs > 0 {
                    meta.set_length(Some(Time::from_secs(duration_secs)));
                }
                let _ = server
                    .properties_changed([Property::Metadata(meta), Property::CanPlay(true), Property::CanPause(true)])
                    .await;
            }
            MprisMsg::Position(secs) => {
                shared
                    .position_us
                    .store(secs.max(0).saturating_mul(1_000_000), Ordering::Relaxed);
                let _ = server
                    .emit(Signal::Seeked {
                        position: Time::from_secs(secs.max(0)),
                    })
                    .await;
            }
            MprisMsg::Volume(v) => {
                let t = v.clamp(0.0, 1.0);
                *shared.volume.lock().unwrap() = t;
                let _ = server.properties_changed([Property::Volume(t)]).await;
            }
        }
                }
            }
    }
    let _ = server.release_bus_name().await;
    log::info!("MPRIS bus released");
}

impl LocalRootInterface for BreezerMpris {
    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }
    async fn quit(&self) -> fdo::Result<()> {
        let _ = self.cmd.try_send(Cmd::QuitApp);
        Ok(())
    }
    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }
    async fn set_fullscreen(&self, _: bool) -> Result<()> {
        Ok(())
    }
    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }
    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(false)
    }
    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }
    async fn identity(&self) -> fdo::Result<String> {
        Ok("Breezer".into())
    }
    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("breezer".into())
    }
    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

impl LocalPlayerInterface for BreezerMpris {
    async fn next(&self) -> fdo::Result<()> {
        let _ = self.cmd.try_send(Cmd::Next);
        Ok(())
    }
    async fn previous(&self) -> fdo::Result<()> {
        let _ = self.cmd.try_send(Cmd::Prev);
        Ok(())
    }
    async fn pause(&self) -> fdo::Result<()> {
        let _ = self.cmd.try_send(Cmd::Toggle);
        Ok(())
    }
    async fn play_pause(&self) -> fdo::Result<()> {
        let _ = self.cmd.try_send(Cmd::Toggle);
        Ok(())
    }
    async fn play(&self) -> fdo::Result<()> {
        let _ = self.cmd.try_send(Cmd::Toggle);
        Ok(())
    }
    async fn stop(&self) -> fdo::Result<()> {
        let _ = self.cmd.try_send(Cmd::Stop);
        Ok(())
    }
    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let cur = self.shared.position_us.load(Ordering::Relaxed);
        let target = ((cur + offset.as_micros()).max(0)) / 1_000_000;
        let _ = self.cmd.try_send(Cmd::Seek(target as f32));
        Ok(())
    }
    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        let _ = self.cmd.try_send(Cmd::Seek(position.as_secs() as f32));
        Ok(())
    }
    async fn open_uri(&self, uri: String) -> fdo::Result<()> {
        log::warn!("MPRIS OpenUri is not supported: {uri}");
        Ok(())
    }
    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(match self.shared.status.load(Ordering::Relaxed) {
            st::PLAYING => PlaybackStatus::Playing,
            st::PAUSED => PlaybackStatus::Paused,
            _ => PlaybackStatus::Stopped,
        })
    }
    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::None)
    }
    async fn set_loop_status(&self, _: LoopStatus) -> Result<()> {
        Ok(())
    }
    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }
    async fn set_rate(&self, _: PlaybackRate) -> Result<()> {
        Ok(())
    }
    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(false)
    }
    async fn set_shuffle(&self, _: bool) -> Result<()> {
        Ok(())
    }
    async fn metadata(&self) -> fdo::Result<Metadata> {
        let g = &self.shared;
        let mut meta = Metadata::new();
        meta.set_trackid(Some(TrackId::NO_TRACK));
        meta.set_title(Some(g.title.lock().unwrap().clone()));
        meta.set_artist(Some(
            g.artist
                .lock()
                .unwrap()
                .split(',')
                .map(|s| s.trim().to_string()),
        ));
        let album = g.album.lock().unwrap().clone();
        if !album.is_empty() {
            meta.set_album(Some(album));
        }
        let art = g.art_url.lock().unwrap().clone();
        if !art.is_empty() {
            meta.set_art_url(Some(art));
        }
        let dur = g.duration_us.load(Ordering::Relaxed);
        if dur > 0 {
            meta.set_length(Some(Time::from_micros(dur)));
        }
        Ok(meta)
    }
    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(*self.shared.volume.lock().unwrap())
    }
    async fn set_volume(&self, volume: Volume) -> Result<()> {
        let _ = self
            .cmd
            .try_send(Cmd::Volume((volume.clamp(0.0, 1.0) * 100.0) as f32));
        Ok(())
    }
    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_micros(
            self.shared.position_us.load(Ordering::Relaxed),
        ))
    }
    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }
    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }
    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(true)
    }
    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}
