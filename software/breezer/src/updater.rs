//! Auto-update via GitHub Releases.
//!
//! Flow (fully automatic, no GitHub round-trip for the user):
//!  1. on launch the app queries the latest release,
//!  2. picks the *replaceable* binary asset for the current platform
//!     (`breezer-linux-x86_64`, `breezer-macos-aarch64`, … — see the CI),
//!  3. downloads it next to a version marker in `~/.config/breezer/updates/`,
//!  4. the UI offers "Install & restart", which atomically replaces the
//!     running executable and relaunches it — a reboot in place.
//!
//! A pending download is also applied automatically on the next launch
//! (`try_apply_pending`), so a closed app still picks up on reboot.

use crate::error::Result;
use semver::Version;
use std::path::{Path, PathBuf};

pub const RELEASES_URL: &str =
    "https://api.github.com/repos/Stananas/Breezer/releases/latest";
pub const UA: &str = "Breezer/0.1 (auto-update)";

/// A release asset (size not needed for binary targeting).
#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub url: String,
}

/// The latest release, with its assets (installers + the raw binaries).
#[derive(Debug, Clone)]
pub struct Release {
    pub version: Version,
    pub notes: String,
    pub url: String,
    pub assets: Vec<Asset>,
}

/// Check whether a newer version is published.
///
/// Returns `Ok(None)` when there is nothing newer (or the feed is unreachable:
/// a 404/network issue simply means "no update").
pub async fn check(http: &reqwest::Client) -> Result<Option<Release>> {
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

    let assets = match body["assets"].as_array() {
        Some(list) => list
            .iter()
            .filter_map(|a| {
                Some(Asset {
                    name: a["name"].as_str()?.to_string(),
                    url: a["browser_download_url"].as_str()?.to_string(),
                })
            })
            .collect(),
        None => Vec::new(),
    };

    Ok(Some(Release {
        version: latest,
        notes: body["body"].as_str().unwrap_or_default().to_string(),
        url: body["html_url"]
            .as_str()
            .unwrap_or(RELEASES_URL)
            .to_string(),
        assets,
    }))
}

/// Stable platform tag used to name the self-update binaries in CI:
/// `breezer-{key}` (see `.github/workflows/release.yml`, matrix artifact).
pub fn platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "aarch64") => "macos-aarch64",
        ("macos", "x86_64") => "macos-x86_64",
        ("windows", "x86_64") => "windows-x86_64",
        _ => "",
    }
}

/// Pick the replaceable binary for the current platform.
///  - exact `breezer-{platform_key}` (or `{key}.exe` on Windows) first,
///  - a Linux AppImage as a fallback.
pub fn pick_binary(assets: &[Asset]) -> Option<Asset> {
    let key = platform_key();
    if key.is_empty() {
        return None;
    }
    let needle = format!("breezer-{key}");
    assets
        .iter()
        .find(|a| a.name == needle || a.name == format!("{needle}.exe"))
        .cloned()
        .or_else(|| {
            if std::env::consts::OS == "linux" {
                assets
                    .iter()
                    .find(|a| a.name.to_ascii_lowercase().ends_with(".appimage"))
                    .cloned()
            } else {
                None
            }
        })
}

/// Updates live under `~/.config/breezer/updates/` (config_dir + /updates).
pub fn updates_dir() -> PathBuf {
    crate::config::config_dir().join("updates")
}

/// Where the incoming binary lands: `updates/breezer-{version}`.
fn binary_path(version: &Version) -> PathBuf {
    updates_dir().join(format!("breezer-{version}"))
}

/// Version marker next to the downloaded binary: `updates/ready.txt`.
fn marker_path() -> PathBuf {
    updates_dir().join("ready.txt")
}

/// Full automatic pipeline: pick the right asset for this platform, download
/// it, sanity-check it, write the version marker. Returns the new version.
#[allow(clippy::too_many_arguments)]
pub async fn prepare_latest_update(
    http: &reqwest::Client,
    rel: &Release,
) -> Result<Option<Version>> {
    let Some(asset) = pick_binary(&rel.assets) else {
        log::info!("update available but no binary for this platform ({})", platform_key());
        return Ok(None);
    };
    let dest = binary_path(&rel.version);
    log::info!("downloading update {} from {}", rel.version, asset.url);
    let size = download(http, &asset.url, &dest).await?;
    if !looks_like_binary(&dest) {
        std::fs::remove_file(&dest).ok();
        return Err(crate::error::Error::Other(
            "downloaded file is not a valid executable".into(),
        ));
    }
    log::info!("update {} downloaded ({} bytes)", rel.version, size);
    std::fs::create_dir_all(updates_dir())?;
    std::fs::write(marker_path(), rel.version.to_string())?;
    Ok(Some(rel.version.clone()))
}

/// Stream `url` into `dest`; returns the number of bytes written.
pub async fn download(http: &reqwest::Client, url: &str, dest: &Path) -> Result<u64> {
    use std::io::Write;
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut resp = http
        .get(url)
        .header("User-Agent", UA)
        .send()
        .await?
        .error_for_status()?;
    let mut out = std::io::BufWriter::new(std::fs::File::create(dest)?);
    let mut total: u64 = 0;
    while let Some(chunk) = resp.chunk().await? {
        out.write_all(&chunk)?;
        total += chunk.len() as u64;
    }
    out.flush()?;
    Ok(total)
}

/// Minimal sanity check: ELF magic + plausible size, so an HTML error page or
/// a truncated download is never installed over the running app.
pub fn looks_like_binary(path: &Path) -> bool {
    use std::io::Read;
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if meta.len() < 1_000_000 {
        return false;
    }
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 4];
    if f.read_exact(&mut magic).is_err() {
        return false;
    }
    magic[0] == 0x7f && magic[1] == b'E' && magic[2] == b'L' && magic[3] == b'F'
}

/// Discover a previously downloaded, not-yet-applied update.
pub fn latest_ready() -> Option<(Version, PathBuf)> {
    let marker = std::fs::read_to_string(marker_path()).ok()?;
    let version = Version::parse(marker.trim()).ok()?;
    let binary = binary_path(&version);
    if !looks_like_binary(&binary) {
        return None;
    }
    Some((version, binary))
}

/// Apply the pending update by replacing the running executable.
/// Returns `true` when an update was applied, `false` when none was pending.
pub fn apply_latest_ready() -> Result<bool> {
    let Some((version, binary)) = latest_ready() else {
        return Ok(false);
    };
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| Version::new(0, 0, 0));
    if version <= current {
        clear_ready();
        return Ok(false);
    }
    let exe = std::env::current_exe()?;
    apply(&exe, &binary)?;
    clear_ready();
    log::info!("applied update {version} over {}", exe.display());
    Ok(true)
}

pub fn clear_ready() {
    std::fs::remove_file(marker_path()).ok();
}

/// Atomically replace `exe` with `new` (Linux/macOS: rename over the running
/// executable is safe; the process keeps its inode until it exits).
#[cfg(unix)]
pub fn apply(exe: &Path, new: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let tmp = exe.with_extension("new");
    std::fs::copy(new, &tmp)?;
    let mut perms = std::fs::metadata(&tmp)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&tmp, perms)?;
    std::fs::rename(&tmp, exe)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn apply(_exe: &Path, _new: &Path) -> Result<()> {
    Err(crate::error::Error::Other(
        "in-place updates are supported on Linux/macOS for now; \
         the new release was kept in ~/.config/breezer/updates/"
            .into(),
    ))
}

/// Relaunch `exe` (the caller then quits the event loop).
pub fn restart(exe: &Path) -> Result<()> {
    std::process::Command::new(exe).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.into(),
            url: format!("https://example.com/{name}"),
        }
    }

    #[test]
    fn semver_compare() {
        assert!(Version::parse("0.2.0").unwrap() > Version::parse("0.1.9").unwrap());
        assert!(Version::parse("0.1.2-beta").unwrap() < Version::parse("0.1.2").unwrap());
    }

    #[test]
    fn pick_linux_binary() {
        let assets = vec![
            asset("breezer_0.2.0_x86_64.deb"),
            asset("breezer_0.2.0_x86_64.AppImage"),
            asset("breezer-linux-x86_64"),
            asset("SHA256SUMS.txt"),
        ];
        let picked = pick_binary(&assets).unwrap();
        assert_eq!(picked.name, "breezer-linux-x86_64");
        // Even without the raw binary, the AppImage is a valid fallback.
        let assets2 = vec![asset("breezer_0.2.0_x86_64.AppImage")];
        assert_eq!(pick_binary(&assets2).unwrap().name, "breezer_0.2.0_x86_64.AppImage");
    }

    #[test]
    fn rejects_html_and_tiny_files() {
        let dir = std::env::temp_dir().join("breezer-upd-test");
        std::fs::create_dir_all(&dir).ok();
        let html = dir.join("bad.html");
        std::fs::write(&html, "<html>too small</html>").ok();
        assert!(!looks_like_binary(&html));
        // A real ELF is accepted (the running test binary itself).
        let exe = std::env::current_exe().unwrap();
        assert!(looks_like_binary(&exe));
    }
}