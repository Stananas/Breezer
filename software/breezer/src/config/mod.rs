//! Persistent configuration, themes and dock-layout profiles.
//!
//! Layout (portable, local):
//!   config dir (dirs::config_dir()) + "/breezer/"
//!     ├── config.json    — ARL, theme id, language, volume, theme overrides
//!     └── layout.json    — dock layout profile

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Config directory for Breezer (`~/.config/breezer` on Linux, app-data on Windows, etc.).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("breezer")
}

// ---------------------------------------------------------------------------
// Theme manifest
// ---------------------------------------------------------------------------

/// A serializable color token spec (hex strings).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PaletteSpec {
    pub bg: String,
    pub surface: String,
    pub surface2: String,
    pub primary: String,
    pub on_primary: String,
    pub accent: String,
    pub text: String,
    pub text_secondary: String,
    pub error: String,
    pub radius: f32,
    pub spacing: f32,
}

impl Default for PaletteSpec {
    fn default() -> Self {
        Self {
            bg: "#0f1012".into(),
            surface: "#17181c".into(),
            surface2: "#1f2127".into(),
            primary: "#a238ff".into(),
            on_primary: "#ffffff".into(),
            accent: "#2a9d8f".into(),
            text: "#f2f3f5".into(),
            text_secondary: "#9aa0a8".into(),
            error: "#e5484d".into(),
            radius: 8.0,
            spacing: 12.0,
        }
    }
}

impl PaletteSpec {
    /// Convert `#rrggbb` (or `#rrggbbaa`) to an rgb tuple; None on parse failure.
    pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
        let h = s.trim().trim_start_matches('#');
        match h.len() {
            6 | 8 => {
                let v = u32::from_str_radix(&h[..6], 16).ok()?;
                Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
            }
            _ => None,
        }
    }
}

/// A theme manifest (palette + optional layout preset + typography tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub palette: PaletteSpec,
    /// Optional layout preset bundled with the theme.
    #[serde(default)]
    pub layout: Option<LayoutProfile>,
}

impl Theme {
    /// Built-in themes, embedded in the binary (kept out of the filesystem).
    const BUILTIN_DARK: &'static str = include_str!("../../../../themes/breezer-dark/theme.json");
    const BUILTIN_LIGHT: &'static str = include_str!("../../../../themes/breezer-light/theme.json");
    const BUILTIN_AMOLED: &'static str =
        include_str!("../../../../themes/breezer-amoled/theme.json");

    /// Load a built-in theme by id (falls back to the dark theme).
    pub fn load_builtin(id: &str) -> Result<Theme> {
        let (json, fallback) = match id {
            "breezer-light" => (Self::BUILTIN_LIGHT, Self::BUILTIN_DARK),
            "breezer-amoled" => (Self::BUILTIN_AMOLED, Self::BUILTIN_DARK),
            _ => (Self::BUILTIN_DARK, Self::BUILTIN_DARK),
        };
        serde_json::from_str(json)
            .or_else(|_| serde_json::from_str(fallback))
            .map_err(Error::from)
    }

    /// Available built-in theme ids.
    pub fn builtin_ids() -> &'static [&'static str] {
        &["breezer-dark", "breezer-light", "breezer-amoled"]
    }

    /// A guaranteed-valid fallback theme (parses from embedded JSON, never fails
    /// since the built-in JSON is a compile-time constant).
    pub fn fallback() -> Theme {
        Theme::load_builtin("breezer-dark").unwrap_or_else(|_| Theme {
            id: "breezer-dark".into(),
            name: "Breezer Dark".into(),
            version: "1.0.0".into(),
            author: String::new(),
            license: "AGPL-3.0-or-later".into(),
            description: String::new(),
            palette: PaletteSpec::default(),
            layout: None,
        })
    }

    /// Apply user overrides (token → hex/value) on top of the theme palette.
    pub fn apply_overrides(&mut self, overrides: &HashMap<String, String>) {
        let p = &mut self.palette;
        for (k, v) in overrides {
            match k.as_str() {
                "bg" => p.bg = v.clone(),
                "surface" => p.surface = v.clone(),
                "surface2" => p.surface2 = v.clone(),
                "primary" => p.primary = v.clone(),
                "on_primary" | "on-primary" => p.on_primary = v.clone(),
                "accent" => p.accent = v.clone(),
                "text" => p.text = v.clone(),
                "text_secondary" | "text-secondary" => p.text_secondary = v.clone(),
                "error" => p.error = v.clone(),
                "radius" => {
                    if let Ok(r) = v.trim().parse::<f32>() {
                        p.radius = r;
                    }
                }
                "spacing" => {
                    if let Ok(s) = v.trim().parse::<f32>() {
                        p.spacing = s;
                    }
                }
                _ => {}
            }
        }
    }

    /// Build the Slint `Palette` struct used by the UI.
    pub fn to_slint(&self) -> crate::Palette {
        let p = &self.palette;
        crate::Palette {
            bg: hex_color(&p.bg),
            surface: hex_color(&p.surface),
            surface2: hex_color(&p.surface2),
            primary: hex_color(&p.primary),
            on_primary: hex_color(&p.on_primary),
            accent: hex_color(&p.accent),
            text: hex_color(&p.text),
            text_secondary: hex_color(&p.text_secondary),
            error: hex_color(&p.error),
            radius: p.radius,
            spacing: p.spacing,
        }
    }
}

fn hex_color(s: &str) -> slint::Color {
    match PaletteSpec::parse_hex(s) {
        Some((r, g, b)) => slint::Color::from_rgb_u8(r, g, b),
        None => slint::Color::from_rgb_u8(15, 16, 18),
    }
}

// ---------------------------------------------------------------------------
// Dock layout profile
// ---------------------------------------------------------------------------

/// Dock layout: which panel occupies which slot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct LayoutProfile {
    /// Panel id in the LEFT slot ("sidebar", "queue", or "" — hidden).
    pub left: String,
    /// Panel id in the RIGHT slot.
    pub right: String,
    /// Panel id in the BOTTOM slot ("player", "queue", or "").
    pub bottom: String,
    /// Density: "compact" | "comfortable" | "spacious".
    #[serde(default = "density_default")]
    pub density: String,
}

fn density_default() -> String {
    "comfortable".into()
}

impl LayoutProfile {
    pub fn default_layout() -> Self {
        Self {
            left: "sidebar".into(),
            right: String::new(),
            bottom: "player".into(),
            density: "comfortable".into(),
        }
    }

    /// Apply a drag & drop: place `panel` into `target` slot ("left"|"right").
    /// Semantics: if the panel currently lives in the other slot, the two
    /// slots swap; if the panel is hidden, it takes the target slot and the
    /// displaced occupant moves to the other slot if free, otherwise is hidden.
    pub fn set_panel(&mut self, panel: &str, target: &str) {
        if !matches!(panel, "sidebar" | "queue") {
            return;
        }
        match target {
            "left" => place_panel(&mut self.left, &mut self.right, panel),
            "right" => place_panel(&mut self.right, &mut self.left, panel),
            _ => {}
        }
    }

    /// Toggle the queue panel visibility (dock to right ↔ hidden).
    pub fn toggle_queue(&mut self) {
        if self.right == "queue" {
            self.right.clear();
        } else if self.left == "queue" {
            self.left.clear();
        } else {
            self.right = "queue".into();
        }
    }

    /// Reset to the default layout.
    pub fn reset(&mut self) {
        *self = Self::default_layout();
    }

    // -- persistence -------------------------------------------------------

    pub fn load() -> LayoutProfile {
        let path = config_dir().join("layout.json");
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|_| Self::default_layout()),
            Err(_) => Self::default_layout(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("layout.json"), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// JSON snapshot for the plugin hook `on_layout_change`.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

fn place_panel(target: &mut String, other: &mut String, panel: &str) {
    if target.as_str() == panel {
        return;
    }
    let displaced = std::mem::replace(target, panel.to_string());
    if other.as_str() == panel {
        // Panel was in `other`: swap the two slots.
        *other = displaced;
    } else if other.is_empty() {
        // Free slot: displaced occupant moves there.
        *other = displaced;
    }
    // Otherwise the displaced occupant is released (hidden).
}

// ---------------------------------------------------------------------------
// App configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Deezer session cookie (secret, stored locally).
    pub arl: Option<String>,
    /// Cached API token derived from the ARL.
    pub api_token: Option<String>,
    pub username: Option<String>,
    pub theme_id: String,
    pub language: String,
    pub volume: f32,
    /// Clock display: "24h" | "12h".
    pub time_format: String,
    /// First-launch onboarding wizard completed.
    pub onboarding_done: bool,
    /// User color overrides on top of the active theme.
    pub theme_overrides: HashMap<String, String>,
    /// Global UI zoom (Ctrl + / Ctrl - / Ctrl 0), 1.0 = default.
    #[serde(default = "zoom_default")]
    pub zoom: f32,
}

fn time_format_default() -> String {
    "24h".into()
}

fn zoom_default() -> f32 {
    1.0
}

impl Default for Config {
    fn default() -> Self {
        Self {
            arl: None,
            api_token: None,
            username: None,
            theme_id: "breezer-dark".into(),
            language: "en".into(),
            volume: 70.0,
            time_format: time_format_default(),
            onboarding_done: false,
            theme_overrides: HashMap::new(),
            zoom: 1.0,
        }
    }
}

impl Config {
    pub fn load() -> Config {
        let path = config_dir().join("config.json");
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("config.json"), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Language helper for the current configuration.
    pub fn i18n(&self) -> crate::i18n::I18n {
        crate::i18n::I18n::from_code(&self.language)
    }
}
