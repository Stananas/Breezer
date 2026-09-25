//! Minimal, dependency-free i18n for Rust-side strings.
//!
//! UI-facing strings inside `.slint` files use Slint's native `@tr()` mechanism
//! (source strings act as the English default). This module covers strings that
//! are produced from Rust code (status messages, errors…).
//!
//! Translations are embedded at compile time: `lang/{en,fr}.json`.

use serde_json::Value;
use std::collections::HashMap;

const EN: &str = include_str!("en.json");
const FR: &str = include_str!("fr.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Fr,
}

impl Lang {
    pub fn from_code(code: &str) -> Lang {
        match code {
            "fr" => Lang::Fr,
            _ => Lang::En,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
        }
    }
}

/// Tiny translation helper. Misses fall back to the key, then to English.
#[derive(Debug, Clone)]
pub struct I18n {
    lang: Lang,
    map: HashMap<String, String>,
}

impl I18n {
    pub fn from_code(code: &str) -> Self {
        Self::new(Lang::from_code(code))
    }

    pub fn new(lang: Lang) -> Self {
        let raw = match lang {
            Lang::En => EN,
            Lang::Fr => FR,
        };
        let map = parse(raw);
        Self { lang, map }
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// Translate `key`, e.g. `t("search.prompt")`.
    pub fn t(&self, key: &str) -> String {
        self.map.get(key).cloned().unwrap_or_else(|| {
            // Fallback: try the English table, then the key itself.
            if self.lang == Lang::En {
                key.to_string()
            } else {
                parse(EN)
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| key.to_string())
            }
        })
    }

    /// Translate with naive `{arg}` substitution, e.g. `t_args("search.results", &[("count", "12"), ("query", "drake")])`.
    pub fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut out = self.t(key);
        for (k, v) in args {
            out = out.replace(&format!("{{{k}}}"), v);
        }
        out
    }
}

fn parse(raw: &str) -> HashMap<String, String> {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.into_iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or(&k).to_string()))
                .collect()
        })
        .unwrap_or_default()
}
