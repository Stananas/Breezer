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
    /// Gateway API token — lives in `results.checkForm` (modern gateway),
    /// with `USER_TOKEN` / `TOKEN` as historical fallbacks.
    pub token: String,
    /// Session id (`results.SESSION_ID`), required for authenticated calls.
    pub session_id: String,
    pub user_id: i64,
    pub username: String,
}

impl DzUserData {
    /// Parse the gateway reply `{ "results": { USER, SESSION_ID, checkForm, … } }`.
    pub fn from_value(body: &serde_json::Value) -> Result<Self, crate::error::Error> {
        let results = body.get("results").cloned().unwrap_or_default();
        let token = results
            .get("checkForm")
            .and_then(|v| v.as_str())
            .or_else(|| results.get("USER_TOKEN").and_then(|v| v.as_str()))
            .or_else(|| results.get("TOKEN").and_then(|v| v.as_str()))
            .unwrap_or_default()
            .to_string();
        let session_id = results
            .get("SESSION_ID")
            .and_then(|v| v.as_str().map(str::to_string))
            .or_else(|| results.get("SESSION_ID").and_then(|v| v.as_i64()).map(|i| i.to_string()))
            .unwrap_or_default();
        let user = results.get("USER").cloned().unwrap_or_default();
        let user_id = user
            .get("USER_ID")
            .and_then(|v| v.as_i64())
            .or_else(|| {
                user.get("USER_ID")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<i64>().ok())
            })
            .unwrap_or(0);
        let username = user
            .get("LOGIN")
            .and_then(|v| v.as_str())
            .or_else(|| user.get("NAME").and_then(|v| v.as_str()))
            .unwrap_or_default()
            .to_string();
        Ok(Self { token, session_id, user_id, username })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gateway_user_data() {
        // Shape of the real `deezer.getUserData` reply (checked against the
        // live gateway): the API token lives in `checkForm`, not `TOKEN`.
        let json = r#"{
          "error": [],
          "results": {
            "USER": { "USER_ID": 123456, "LOGIN": "bob", "NAME": "Bob",
                      "OPTIONS": { "license_token": "AAAAlicense" } },
            "SESSION_ID": "frcb7a74850a29abd7e2b77df1211fbb8ee680c6",
            "USER_TOKEN": "user.token.value",
            "TOKEN": "legacy.token.value",
            "checkForm": "5.HXyAeGX2nhJkdoKp1Zk9dl-YfK0Sjn"
          }
        }"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let d = DzUserData::from_value(&v).unwrap();
        assert_eq!(d.token, "5.HXyAeGX2nhJkdoKp1Zk9dl-YfK0Sjn");
        assert_eq!(
            d.session_id,
            "frcb7a74850a29abd7e2b77df1211fbb8ee680c6"
        );
        assert_eq!(d.user_id, 123456);
        assert_eq!(d.username, "bob");
    }

    #[test]
    fn parse_fake_arl_shape() {
        // A fake/expired ARL still answers 200 with USER_ID = 0 and no login —
        // the auth layer must reject it.
        let json = r#"{"error":[],"results":{"USER":{"USER_ID":0,"LOGIN":"","NAME":"","OPTIONS":{}},"SESSION_ID":"x","checkForm":"5.x"}}"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let d = DzUserData::from_value(&v).unwrap();
        assert_eq!(d.user_id, 0);
        assert!(d.username.is_empty());
    }
}
