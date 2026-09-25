//! Audio engine, stream decryption and playback queue.

pub mod decrypt;
pub mod engine;
pub mod queue;

pub use engine::Engine;
pub use queue::PlayQueue;

/// Shared audio types (rodio re-exports).
pub use rodio::source::{SineWave, Source};

/// A prepared, decodable stream for one track (v0.2 fills this in).
#[derive(Debug, Clone)]
pub struct StreamHandle {
    pub track_id: u64,
    pub url: String,
    pub decryption_key: Option<[u8; 16]>,
    pub quality: String,
}