//! Deezer HTTP client.
//!
//! Two surfaces:
//!  - **Public API** (`api.deezer.com`) — search and metadata, no auth needed.
//!  - **Authenticated gateway** (`www.deezer.com/ajax/gw-light.php`) — ARL-based
//!    session bootstrap that yields the API token and account info.
//!
//! Everything is behind one [`ApiClient`] and fully async.

pub mod models;

use crate::error::{Error, Result};
use models::{DzSearchResponse, Track};

/// Deezer gateway endpoint (authenticated, cookie-based).
pub const GW_LIGHT_URL: &str = "https://www.deezer.com/ajax/gw-light.php";
/// Public REST endpoint.
pub const API_BASE: &str = "https://api.deezer.com";

/// Result of an ARL-based login.
#[derive(Debug, Clone, Default)]
pub struct AuthSession {
    pub api_token: String,
    pub username: String,
    pub user_id: i64,
    /// Gateway session id (SESSION_ID) for authenticated calls.
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    pub http: reqwest::Client,
    arl: Option<String>,
    api_token: Option<String>,
    /// Gateway session id (SESSION_ID), required for authenticated gw calls.
    session_id: Option<String>,
}

/// Real browser User-Agent: Deezer's gateway refuses or reduces responses to
/// non-browser clients (proven flow of the tui-dzr client).
pub const BROWSER_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

impl ApiClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::ClientBuilder::new()
            .user_agent(BROWSER_UA)
            .cookie_store(true)
            .build()?;
        Ok(Self { http, arl: None, api_token: None, session_id: None })
    }

    /// Build from the stored configuration.
    pub fn from_config(cfg: &crate::config::Config) -> Result<Self> {
        let client = Self::new()?;
        Ok(Self { arl: cfg.arl.clone(), api_token: cfg.api_token.clone(), ..client })
    }

    pub fn is_authenticated(&self) -> bool {
        self.arl.is_some()
    }

    // ---------------------------------------------------------------------
    // Auth
    // ---------------------------------------------------------------------

    /// Validate an ARL cookie against Deezer and derive the API token.
    ///
    /// Flow matched against the tui-dzr client (known-good against the current
    /// gateway): POST `{}` with `Content-Type: application/json`, a real
    /// browser User-Agent, the `arl` cookie, and the token read from
    /// `results.checkForm`.
    pub async fn auth_with_arl(&self, arl: &str) -> Result<AuthSession> {
        let arl = arl.trim().to_string();
        if arl.len() < 32 {
            return Err(Error::Auth("ARL looks too short to be valid".into()));
        }
        let resp = self
            .http
            .post(GW_LIGHT_URL)
            .query(&[
                ("method", "deezer.getUserData"),
                ("api_version", "1.0"),
                ("api_token", ""),
            ])
            .header("Cookie", format!("arl={arl}"))
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await?;
        log::debug!("gw-light responded HTTP {}", resp.status());

        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            return Err(Error::Auth(format!("gateway returned HTTP {status}")));
        }
        if let Some(err) = body.get("error").and_then(|e| e.as_array()) {
            if !err.is_empty() {
                let msg = err
                    .first()
                    .and_then(|m| m.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown gateway error")
                    .to_string();
                return Err(Error::Auth(msg));
            }
        }

        let inner = models::DzUserData::from_value(&body)?;
        // A valid account always has a user id (and a session). A fake/expired
        // ARL still receives a gateway response, but with USER_ID = 0.
        if inner.user_id == 0 && inner.username.is_empty() {
            return Err(Error::Auth("ARL invalid or expired".into()));
        }
        log::debug!(
            "gw-light session OK (user_id={}, session present={})",
            inner.user_id,
            !inner.session_id.is_empty()
        );
        Ok(AuthSession {
            api_token: inner.token.clone(),
            username: inner.username.clone(),
            user_id: inner.user_id,
            session_id: inner.session_id.clone(),
        })
    }

    /// Update the client's session after a fresh login.
    pub fn set_session(&mut self, arl: String, session: &AuthSession) {
        self.arl = Some(arl);
        self.api_token = Some(session.api_token.clone());
        self.session_id = Some(session.session_id.clone());
    }

    // ---------------------------------------------------------------------
    // Search & metadata (public API)
    // ---------------------------------------------------------------------

    /// Search tracks on the public API. `limit` ≤ 100.
    pub async fn search_tracks(&self, query: &str, limit: u32) -> Result<Vec<Track>> {
        let resp = self
            .http
            .get(format!("{API_BASE}/search"))
            .query(&[("q", query), ("limit", &limit.to_string())])
            .send()
            .await?
            .error_for_status()?;
        let body: DzSearchResponse = resp.json().await?;
        Ok(body.data)
    }

    /// Chart tracks (used to seed the queue at first launch).
    pub async fn chart(&self, limit: u32) -> Result<Vec<Track>> {
        let resp = self
            .http
            .get(format!("{API_BASE}/chart/0/tracks"))
            .query(&[("limit", &limit.to_string())])
            .send()
            .await?
            .error_for_status()?;
        let body: DzSearchResponse = resp.json().await?;
        Ok(body.data)
    }

    /// Playlists of the signed-in user.
    pub async fn user_playlists(
        &self,
        user_id: i64,
        limit: u32,
    ) -> Result<Vec<models::Playlist>> {
        let resp = self
            .http
            .get(format!("{API_BASE}/user/{user_id}/playlists"))
            .query(&[("limit", &limit.to_string())])
            .send()
            .await?
            .error_for_status()?;
        let body: models::DzPlaylistResponse = resp.json().await?;
        Ok(body.data)
    }
}