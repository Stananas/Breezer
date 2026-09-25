//! Audio output engine.
//!
//! Built on rodio 0.22 (`MixerDeviceSink` + `Player`). Designed to be lazy:
//! if no audio device is available the engine degrades gracefully instead of
//! panicking (important for CI and headless environments).
//!
//! v0.1: transport + controls (volume, play/pause) + audio self-test.
//! v0.2: real Deezer streams get decoded via `streaming_source()` and are
//! appended to the `Player` (which accepts any `rodio::Source`).

use crate::player::decrypt::StreamDecryptor;
use rodio::source::SineWave;
use rodio::Source;
use std::num::NonZeroU32;
use std::time::Duration;

pub struct Engine {
    /// Kept alive for the whole session — owns the output stream.
    _handle: Option<rodio::MixerDeviceSink>,
    player: Option<rodio::Player>,
    volume: f32,
    playing: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(handle) => {
                let player = rodio::Player::connect_new(handle.mixer());
                let engine = Self {
                    _handle: Some(handle),
                    player: Some(player),
                    volume: 0.7,
                    playing: false,
                };
                engine.apply_volume();
                engine
            }
            Err(e) => {
                log::warn!("no audio output device available: {e}");
                Self { _handle: None, player: None, volume: 0.7, playing: false }
            }
        }
    }

    pub fn has_device(&self) -> bool {
        self.player.is_some()
    }

    // -- controls ----------------------------------------------------------

    /// Play/pause the current source.
    pub fn toggle(&mut self) {
        if let Some(p) = &self.player {
            if p.is_paused() {
                p.play();
            } else {
                p.pause();
            }
            self.playing = !p.is_paused();
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.apply_volume();
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    fn apply_volume(&self) {
        if let Some(p) = &self.player {
            p.set_volume(self.volume);
        }
    }

    /// Seek within the current source (if the source supports it).
    pub fn seek(&self, seconds: f32) {
        if let Some(p) = &self.player {
            let _ = p.try_seek(Duration::from_secs_f32(seconds.max(0.0)));
        }
    }

    pub fn stop(&mut self) {
        if let Some(p) = &self.player {
            p.stop();
        }
        self.playing = false;
    }

    // -- sources -----------------------------------------------------------

    /// Queue a decoded `Source` and start playing it.
    pub fn play_source<S>(&mut self, source: S)
    where
        S: rodio::Source + Send + 'static,
    {
        if let Some(p) = &self.player {
            p.append(source);
            p.play();
            self.playing = true;
        }
    }

    /// Play a fully decrypted MP3 stream (memory buffer).
    ///
    /// Flow (mirroring tui-dzr): decrypt the media.get_url payload
    /// (Blowfish-CBC stripe) and feed the bytes to rodio's symphonia decoder.
    pub fn play_mp3_bytes(&mut self, bytes: Vec<u8>) -> crate::error::Result<()> {
        let p = self
            .player
            .as_ref()
            .ok_or_else(|| crate::error::Error::Other("no audio output device".into()))?;
        let cursor = std::io::Cursor::new(bytes);
        let decoder = rodio::Decoder::new(cursor).map_err(|e| {
            crate::error::Error::Other(format!("failed to decode MP3 stream: {e}"))
        })?;
        p.stop();
        p.append(decoder);
        p.set_volume(self.volume);
        p.play();
        self.playing = true;
        log::info!("streaming: MP3 decoder started");
        Ok(())
    }

    /// v0.2: wrap a Deezer stream (decrypted, chunked HTTP) into a
    /// `rodio::Source`. `_decryptor` keeps the decryption scheme isolated.
    pub fn streaming_source(
        &self,
        _handle: &crate::player::StreamHandle,
        _decryptor: Box<dyn StreamDecryptor>,
    ) -> crate::error::Result<()> {
        Err(crate::error::Error::Unimplemented(
            "Deezer stream playback lands in v0.2 (needs ARL session)".into(),
        ))
    }

    // -- self-test ---------------------------------------------------------

    /// Play a short sine tone to verify the output stack works end-to-end.
    pub fn play_self_test(&self) -> bool {
        if let Some(handle) = &self._handle {
            let mixer = handle.mixer();
            let tone = SineWave::new(523.25) // C5
                .amplify(0.15)
                .take_duration(Duration::from_millis(900));
            mixer.add(tone);
            true
        } else {
            false
        }
    }

    /// 44100 is the canonical sample rate used by all built-in sources.
    #[allow(dead_code)]
    fn sample_rate() -> NonZeroU32 {
        NonZeroU32::new(44100).unwrap()
    }
}