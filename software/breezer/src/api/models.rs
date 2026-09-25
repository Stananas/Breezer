//! Serde models for the Deezer API (public + gateway).

use serde::Deserialize;

/// A track as returned by `api.deezer.com/{search,chart}/…`.
#[derive(Debug, Clone, Deserialize)]
pub struct Track {
    #[serde(rename = "id")]
    pub id: u64,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "duration")]
    pub duration: u32,
    #[serde(default, rename = "artist")]
    pub artist: Artist,
    #[serde(default, rename = "album")]
    pub album: Album,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Artist {
    #[serde(rename = "name")]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Album {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(default, rename = "cover_medium")]
    pub cover_medium: String,
    #[serde(default, rename = "cover_big")]
    pub cover_big: String,
}

#[derive(Debug, Deserialize)]
pub struct DzSearchResponse {
    pub data: Vec<Track>,
}

/// A playlist (user playlists endpoint).
#[derive(Debug, Clone, Deserialize)]
pub struct Playlist {
    #[serde(rename = "id")]
    pub id: u64,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(default)]
    pub picture_medium: String,
    #[serde(default)]
    pub nb_tracks: u32,
}

#[derive(Debug, Deserialize)]
pub struct DzPlaylistResponse {
    pub data: Vec<Playlist>,
}

/// A playlist detail (kept for the playlists view).
#[derive(Debug, Deserialize)]
pub struct DzPlaylistDetail {
    pub tracks: DzTrackItems,
}

#[derive(Debug, Deserialize)]
pub struct DzTrackItems {
    pub data: Vec<Track>,
}

// ---------------------------------------------------------------------------
// Gateway (`gw-light.php`) data
// ---------------------------------------------------------------------------

/// Minimal typed view over `deezer.getUserData` results.
#[derive(Debug, Clone, Default)]
pub struct DzUserData {
    pub token: String,
    pub user_id: i64,
    pub username: String,
}

impl DzUserData {
    /// Parse the gateway reply `{ "results": { TOKEN, USER: {...} } }`.
    pub fn from_value(body: &serde_json::Value) -> Result<Self, crate::error::Error> {
        let results = body.get("results").cloned().unwrap_or_default();
        let token = results
            .get("TOKEN")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let user = results.get("USER").cloned().unwrap_or_default();
        let user_id = user.get("USER_ID").and_then(|v| v.as_i64()).unwrap_or(0);
        let username = user
            .get("LOGIN")
            .and_then(|v| v.as_str())
            .or_else(|| user.get("NAME").and_then(|v| v.as_str()))
            .unwrap_or_default()
            .to_string();
        Ok(Self { token, user_id, username })
    }
}