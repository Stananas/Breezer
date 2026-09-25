//! Auto-update via GitHub Releases.

use crate::error::Result;
use semver::Version;

/// Information about an available update.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: Version,
    pub notes: String,
    pub url: String,
}

/// Current release feed. (The repository URL is a placeholder until the repo
/// is published — a 404 simply reports "no update", which is correct for now.)
const RELEASES_URL: &str =
    "https://api.github.com/repos/Breezer-App/breezer/releases/latest";
const UA: &str = "Breezer/0.1 (auto-update)";

/// Check whether a newer version is published on GitHub.
pub async fn check(http: &reqwest::Client) -> Result<Option<UpdateInfo>> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| crate::error::Error::Other(format!("bad package version: {e}")))?;

    let resp = match http.get(RELEASES_URL).header("User-Agent", UA).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("update check failed (network): {e}");
            return Ok(None);
        }
    };
    if !resp.status().is_success() {
        log::info!("update check: no release feed (HTTP {})", resp.status());
        return Ok(None);
    }
    let body: serde_json::Value = resp.json().await?;

    let tag = body["tag_name"].as_str().unwrap_or_default().trim_start_matches('v');
    let latest = match Version::parse(tag) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    if latest <= current {
        return Ok(None);
    }

    let url = body["html_url"]
        .as_str()
        .unwrap_or(RELEASES_URL)
        .to_string();
    let notes = body["body"].as_str().unwrap_or_default().to_string();
    Ok(Some(UpdateInfo { version: latest, notes, url }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_compare() {
        assert!(Version::parse("0.2.0").unwrap() > Version::parse("0.1.9").unwrap());
        // Pre-releases sort before the release they precede.
        assert!(Version::parse("0.1.2-beta").unwrap() < Version::parse("0.1.2").unwrap());
    }
}