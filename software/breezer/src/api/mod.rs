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
/// Signed media URL endpoint (login-based streaming, see tui-dzr flow).
pub const MEDIA_URL_API: &str = "https://media.deezer.com/v1/get_url";

/// Result of an ARL-based login.
#[derive(Debug, Clone, Default)]
pub struct AuthSession {
    pub api_token: String,
    pub username: String,
    pub user_id: i64,
    /// Gateway session id (SESSION_ID) for authenticated calls.
    pub session_id: String,
    /// Gateway license token (USER.OPTIONS.license_token) for media.get_url.
    pub license_token: String,
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    pub http: reqwest::Client,
    arl: Option<String>,
    api_token: Option<String>,
    /// Gateway session id (SESSION_ID), required for authenticated gw calls.
    session_id: Option<String>,
    /// Gateway license token, required for media.get_url streaming.
    license_token: Option<String>,
    /// Signed-in user id (used by pageProfile etc.).
    user_id: Option<i64>,
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
        Ok(Self { http, arl: None, api_token: None, session_id: None, license_token: None, user_id: None })
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
            license_token: inner.license_token.clone(),
        })
    }

    /// Update the client's session after a fresh login.
    pub fn set_session(&mut self, arl: String, session: &AuthSession) {
        self.arl = Some(arl);
        self.api_token = Some(session.api_token.clone());
        self.session_id = Some(session.session_id.clone());
        self.license_token = Some(session.license_token.clone());
        self.user_id = Some(session.user_id);
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

    // ---------------------------------------------------------------------
    // Streaming (flow matched with the tui-dzr client)
    //   pageTrack → TRACK_TOKEN → media.get_url (license_token, BF_CBC_STRIPE)
    //   → signed CDN URL → download encrypted bytes (decrypted by the player)
    // ---------------------------------------------------------------------

    /// Authenticated gateway call (JSON body + `arl`/`sid` cookies + api_token).
    async fn gateway_call(&self, method: &str, payload: serde_json::Value) -> Result<serde_json::Value> {
        let api_token = self
            .api_token
            .as_deref()
            .ok_or_else(|| Error::Auth("not authenticated — connect with your ARL first".into()))?;
        let arl = self.arl.as_deref().unwrap_or_default();
        let sid = self.session_id.as_deref().unwrap_or_default();
        self.http
            .post(GW_LIGHT_URL)
            .query(&[
                ("method", method),
                ("api_version", "1.0"),
                ("api_token", api_token),
                ("input", "3"),
            ])
            .header("Cookie", format!("arl={arl}; sid={sid}"))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await
            .map_err(Error::from)
    }

    /// Resolve the `TRACK_TOKEN` for a track id (`deezer.pageTrack`).
    pub async fn track_token(&self, track_id: u64) -> Result<String> {
        let resp = self
            .gateway_call("deezer.pageTrack", serde_json::json!({ "sng_id": track_id.to_string() }))
            .await?;
        let data = resp
            .get("results")
            .and_then(|r| r.get("DATA"))
            .or_else(|| resp.get("results"));
        data.and_then(|d| d.get("TRACK_TOKEN"))
            .and_then(|t| t.as_str())
            .map(str::to_string)
            .ok_or_else(|| Error::Other("pageTrack response missing TRACK_TOKEN".into()))
    }

    /// Ask media.get_url for a signed stream URL (MP3_128, Blowfish-CBC stripe).
    pub async fn media_url(&self, track_token: &str) -> Result<String> {
        let license = self
            .license_token
            .as_deref()
            .ok_or_else(|| Error::Auth("no license token — reconnect with your ARL".into()))?;
        let arl = self.arl.as_deref().unwrap_or_default();
        let sid = self.session_id.as_deref().unwrap_or_default();

        let payload = serde_json::json!({
            "license_token": license,
            "media": [{
                "type": "FULL",
                "formats": [{ "cipher": "BF_CBC_STRIPE", "format": "MP3_128" }]
            }],
            "track_tokens": [track_token]
        });

        let resp: serde_json::Value = self
            .http
            .post(MEDIA_URL_API)
            .header("Cookie", format!("arl={arl}; sid={sid}"))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        resp["data"][0]["media"][0]["sources"][0]["url"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| Error::Other("media.get_url response missing signed URL".into()))
    }

    /// Full streaming fetch: token → signed URL → encrypted MP3 bytes.
    pub async fn stream_encrypted_mp3(&self, track_id: u64) -> Result<Vec<u8>> {
        let token = self.track_token(track_id).await?;
        let url = self.media_url(&token).await?;
        let bytes = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes.to_vec())
    }

    // ---------------------------------------------------------------------
    // Playlists (flow matched with the tui-dzr client)
    //   pageProfile (tab=playlists) → list; pagePlaylist (paged) → tracks
    // ---------------------------------------------------------------------

    /// List the signed-in user's playlists (`deezer.pageProfile`).
    pub async fn playlists(&self) -> Result<Vec<models::PlaylistMeta>> {
        let profile_id = self
            .user_id
            .ok_or_else(|| Error::Auth("not authenticated — connect with your ARL first".into()))?;
        let payload = serde_json::json!({
            "profile_id": profile_id,
            "user_id": profile_id,
            "USER_ID": profile_id,
            "tab": "playlists",
            "nb": 40,
        });
        let resp = self.gateway_call("deezer.pageProfile", payload).await?;
        let mut out = Vec::new();
        let lists = resp["results"]["TAB"]["playlists"]["data"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for item in lists {
            let id = item["PLAYLIST_ID"]
                .as_i64()
                .or_else(|| item["id"].as_i64())
                .or_else(|| item["PLAYLIST_ID"].as_str().and_then(|s| s.parse::<i64>().ok()))
                .or_else(|| item["id"].as_str().and_then(|s| s.parse::<i64>().ok()))
                .unwrap_or(0);
            if id == 0 {
                continue;
            }
            let title = item["TITLE"]
                .as_str()
                .or_else(|| item["title"].as_str())
                .unwrap_or("Untitled playlist")
                .to_string();
            let picture = item["PLAYLIST_PICTURE"]
                .as_str()
                .or_else(|| item["PICTURE"].as_str())
                .or_else(|| item["picture"].as_str())
                .unwrap_or_default()
                .to_string();
            let count = item["NB_SONG"]
                .as_u64()
                .or_else(|| item["nb_tracks"].as_u64())
                .unwrap_or(0) as u32;
            out.push(models::PlaylistMeta { id: id as u64, title, picture_hash: picture, count });
        }
        Ok(out)
    }

    /// Fetch all tracks of a playlist (`deezer.pagePlaylist`, paged by 200).
    pub async fn playlist_tracks(&self, playlist_id: &str) -> Result<Vec<models::Track>> {
        let mut out: Vec<models::Track> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let page_size = 200usize;
        let mut start = 0usize;

        for _page in 0..10 {
            let payload = serde_json::json!({
                "playlist_id": playlist_id,
                "lang": "en",
                "header": true,
                "start": start,
                "nb": page_size,
            });
            let resp = self.gateway_call("deezer.pagePlaylist", payload).await?;
            let tracks = resp["results"]["SONGS"]["data"]
                .as_array()
                .or_else(|| resp["results"]["DATA"]["SONGS"]["data"].as_array())
                .or_else(|| resp["results"]["tracks"]["data"].as_array())
                .or_else(|| resp["results"]["TRACKS"]["data"].as_array())
                .or_else(|| resp["results"]["tracks"].as_array())
                .or_else(|| resp["results"]["SONGS"].as_array())
                .cloned()
                .unwrap_or_default();

            let before = out.len();
            for track in tracks {
                let id = track["SNG_ID"]
                    .as_u64()
                    .or_else(|| track["SNG_ID"].as_str().and_then(|s| s.parse::<u64>().ok()))
                    .or_else(|| track["id"].as_u64())
                    .or_else(|| track["id"].as_str().and_then(|s| s.parse::<u64>().ok()))
                    .unwrap_or(0);
                if id == 0 || !seen.insert(id) {
                    continue;
                }
                let title = track["SNG_TITLE"]
                    .as_str()
                    .or_else(|| track["title"].as_str())
                    .unwrap_or("Unknown track")
                    .to_string();
                let artist = track["ART_NAME"]
                    .as_str()
                    .unwrap_or("Unknown artist")
                    .to_string();
                let album_title = track["ALB_TITLE"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let cover = track["ALB_PICTURE"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(|hash| {
                        format!("https://e-cdns-images.dzcdn.net/images/cover/{hash}/250x250-000000-80-0-0.jpg")
                    })
                    .unwrap_or_default();
                let duration = track["DURATION"]
                    .as_u64()
                    .or_else(|| {
                        track["DURATION"]
                            .as_str()
                            .and_then(|s| s.parse::<u64>().ok())
                    })
                    .unwrap_or(0) as u32;
                out.push(models::Track {
                    id,
                    title,
                    duration,
                    artist: models::Artist { name: artist },
                    album: models::Album { title: album_title, cover_medium: cover, cover_big: String::new() },
                });
            }
            if out.len() == before {
                break;
            }
            start += page_size;
        }
        Ok(out)
    }

    // ---------------------------------------------------------------------
    // "Last played" (cross-device resume, `pageProfile tab=history`)
    // ---------------------------------------------------------------------

    /// Recently played tracks from Deezer's listening history (newest first).
    pub async fn recent_played(&self, limit: usize) -> Result<Vec<models::Track>> {
        let uid = self
            .user_id
            .ok_or_else(|| Error::Auth("not authenticated — connect with your ARL first".into()))?;
        let resp = self
            .gateway_call(
                "deezer.pageProfile",
                serde_json::json!({
                    "profile_id": uid,
                    "user_id": uid,
                    "USER_ID": uid,
                    "tab": "history",
                }),
            )
            .await?;
        let items = resp["results"]["TAB"]["history"]["data"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        Ok(items
            .into_iter()
            .filter_map(parse_history_track)
            .take(limit)
            .collect())
    }

    /// Last played track — first item of the listening history.
    pub async fn last_played(&self) -> Result<Option<models::Track>> {
        Ok(self.recent_played(1).await?.into_iter().next())
    }

    /// Track duration via the public API (history items often lack DURATION).
    pub async fn track_duration(&self, id: u64) -> Option<u32> {
        let url = format!("{API_BASE}/track/{id}");
        match self.http.get(&url).send().await {
            Ok(r) if r.status().is_success() => {
                r.json::<serde_json::Value>().await.ok().and_then(|v| {
                    v["duration"].as_u64().map(|d| d as u32)
                })
            }
            _ => None,
        }
    }
}

/// Parse a Deezer "history" item (JSON blob) into a `Track`.
/// Covers come from the `ALB_PICTURE` hash (same CDN pattern as the rest).
fn parse_history_track(v: serde_json::Value) -> Option<models::Track> {
    let id = v["SNG_ID"]
        .as_u64()
        .or_else(|| v["SNG_ID"].as_str().and_then(|s| s.parse::<u64>().ok()))
        .unwrap_or(0);
    if id == 0 {
        return None;
    }
    let title = v["SNG_TITLE"].as_str().unwrap_or("Unknown track").to_string();
    let artist = v["ART_NAME"].as_str().unwrap_or("Unknown artist").to_string();
    let cover = v["ALB_PICTURE"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|h| {
            format!("https://e-cdns-images.dzcdn.net/images/cover/{h}/250x250-000000-80-0-0.jpg")
        })
        .unwrap_or_default();
    let duration = v["DURATION"]
        .as_u64()
        .or_else(|| v["DURATION"].as_str().and_then(|s| s.parse::<u64>().ok()))
        .unwrap_or(0) as u32;
    Some(models::Track {
        id,
        title,
        duration,
        artist: models::Artist { name: artist },
        album: models::Album {
            title: String::new(),
            cover_medium: cover,
            cover_big: String::new(),
        },
    })
}