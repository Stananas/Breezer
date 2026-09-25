//! `breezer-plugin-api`: the stable public plugin interface.
//!
//! This crate intentionally has **zero dependencies** so plugins can link
//! against it without pulling the whole application. It is the ABI contract
//! that future dynamic loaders (sidecar JSON-RPC, WASM sandbox) will satisfy.

/// Metadata about the currently playing track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMeta {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
}

/// A menu entry contributed by a plugin (shell extension point, v0.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// Stable identifier (used when the item is triggered).
    pub id: String,
    /// Display label (can be i18n'd by the app).
    pub label: String,
    /// Optional hint / accessibility description.
    pub hint: String,
}

impl MenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self { id: id.into(), label: label.into(), hint: String::new() }
    }
}

/// The trait every Breezer plugin implements.
///
/// # Stability
/// This trait is the public ABI of Breezer plugins. Breaking changes are only
/// allowed on major version bumps. Do not add crate-private types to signatures.
pub trait Plugin: Send + Sync {
    /// Unique, stable identifier, e.g. `"lyrics-fetcher"`.
    fn id(&self) -> &'static str;
    /// Human-readable name.
    fn name(&self) -> &'static str;
    /// Semver string of the plugin.
    fn version(&self) -> &'static str;
    /// One-line description.
    fn description(&self) -> &'static str {
        ""
    }

    // -- Hooks -------------------------------------------------------------

    /// Called when the playing track changes.
    fn on_track_change(&self, _track: &TrackMeta) {}

    /// Called when the active theme changes.
    fn on_theme_change(&self, _theme_id: &str) {}

    /// Called when the dock layout changes (JSON-encoded layout profile).
    fn on_layout_change(&self, _layout_json: &str) {}

    /// Called when the Deezer auth state changes.
    fn on_auth_change(&self, _logged_in: bool, _username: &str) {}

    // -- Shell extension points -------------------------------------------

    /// Contributions to the app menu (extensions toolbar).
    fn menu_items(&self) -> Vec<MenuItem> {
        Vec::new()
    }
}