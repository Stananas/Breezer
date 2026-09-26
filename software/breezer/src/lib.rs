//! Breezer — ultra-lightweight native Deezer client.

pub mod api;
pub mod app;
pub mod config;
pub mod error;
pub mod i18n;
pub mod media;
pub mod player;
pub mod plugins;
pub mod updater;

#[cfg(target_os = "linux")]
mod mpris;

// Embeds the compiled Slint UI (built by `build.rs` from `src/ui/`).
slint::include_modules!();
