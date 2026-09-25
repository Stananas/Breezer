//! Plugin registry and the built-in system plugin.
//!
//! v0.1: plugins are compiled-in and registered here; the public contract lives
//! in the `breezer-plugin-api` crate (see docs/PLUGINS.md). Dynamic loading
//! (sidecar JSON-RPC / WASM) is a roadmap item that instantiates the same trait.

use breezer_plugin_api::{MenuItem, Plugin, TrackMeta};

/// Holds every active plugin and dispatches lifecycle hooks.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    /// Registry with the built-in system plugin.
    pub fn with_system() -> Self {
        Self {
            plugins: vec![Box::<SystemPlugin>::default()],
        }
    }

    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    // -- hook dispatch -----------------------------------------------------

    pub fn notify_track(&self, track: &TrackMeta) {
        for p in &self.plugins {
            p.on_track_change(track);
        }
    }

    pub fn notify_theme(&self, theme_id: &str) {
        for p in &self.plugins {
            p.on_theme_change(theme_id);
        }
    }

    pub fn notify_layout(&self, layout_json: &str) {
        for p in &self.plugins {
            p.on_layout_change(layout_json);
        }
    }

    pub fn notify_auth(&self, logged_in: bool, username: &str) {
        for p in &self.plugins {
            p.on_auth_change(logged_in, username);
        }
    }

    /// Aggregate menu contributions from all plugins.
    pub fn menu_items(&self) -> Vec<MenuItem> {
        self.plugins.iter().flat_map(|p| p.menu_items()).collect()
    }
}

/// Built-in plugin: declares the app menu entry.
#[derive(Default)]
pub struct SystemPlugin;

impl Plugin for SystemPlugin {
    fn id(&self) -> &'static str {
        "system"
    }
    fn name(&self) -> &'static str {
        "Breezer System"
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn description(&self) -> &'static str {
        "Built-in system plugin (app menu)."
    }
    fn menu_items(&self) -> Vec<MenuItem> {
        vec![MenuItem::new("about", "About Breezer")]
    }
}

/// Convert a UI track into the plugin `TrackMeta`.
pub fn track_meta(track: &crate::TrackInfo) -> TrackMeta {
    TrackMeta {
        id: track.id as i64,
        title: track.title.to_string(),
        artist: track.artist.to_string(),
        album: track.album.to_string(),
    }
}
