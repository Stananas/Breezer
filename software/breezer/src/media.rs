//! Cover image handling: remote fetch + decode + a small bounded cache.
//!
//! `slint::Image` is not `Send`, so the cache stores **decoded RGBA bytes**
//! (Send) and the `Image` is constructed on the UI thread. The cache itself is
//! shared with the services task through an `Arc`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// (width, height, RGBA bytes)
type CoverData = (u32, u32, Vec<u8>);

/// Cheap-to-clone handle to the shared cover cache.
#[derive(Clone)]
pub struct Covers(Arc<CoversInner>);

struct CoversInner {
    cache: Mutex<HashMap<String, CoverData>>,
    client: reqwest::Client,
}

impl Covers {
    pub fn new(client: reqwest::Client) -> Self {
        Self(Arc::new(CoversInner { cache: Mutex::new(HashMap::new()), client }))
    }

    /// Ensure every URL is fetched (+decoded) and cached. Missing/failed
    /// covers become a placeholder on read.
    pub async fn ensure(&self, urls: &[String]) {
        for url in urls {
            if self.cached(url).is_some() {
                continue;
            }
            let data = match self.0.client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                    Ok(bytes) => decode_jpeg_to_rgba(&bytes),
                    Err(_) => None,
                },
                _ => None,
            };
            if let Some(d) = data {
                if let Ok(mut cache) = self.0.cache.lock() {
                    cache.insert(url.clone(), d);
                }
            }
        }
    }

    /// Get a cover as a Slint image (UI thread). Falls back to a placeholder.
    pub fn image(&self, url: &str) -> slint::Image {
        match self.cached(url) {
            Some((w, h, rgba)) => {
                let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
                buf.make_mut_bytes().copy_from_slice(&rgba);
                slint::Image::from_rgba8(buf)
            }
            None => placeholder_cover(),
        }
    }

    fn cached(&self, url: &str) -> Option<CoverData> {
        self.0.cache.lock().ok()?.get(url).cloned()
    }

    /// Drop all cached covers (new search).
    pub fn clear(&self) {
        if let Ok(mut cache) = self.0.cache.lock() {
            cache.clear();
        }
    }
}

/// Decode a JPEG payload into RGBA bytes.
fn decode_jpeg_to_rgba(bytes: &[u8]) -> Option<CoverData> {
    use jpeg_decoder::Decoder;
    let mut dec = Decoder::new(bytes);
    let pixels = dec.decode().ok()?;
    let info = dec.info()?;
    let (w, h) = (info.width as usize, info.height as usize);
    if pixels.len() < w * h * 3 {
        return None;
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for px in pixels.chunks_exact(3).take(w * h) {
        rgba.push(px[0]);
        rgba.push(px[1]);
        rgba.push(px[2]);
        rgba.push(255);
    }
    Some((w as u32, h as u32, rgba))
}

/// A solid-color 250x250 placeholder cover.
pub fn placeholder_cover() -> slint::Image {
    let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(250, 250);
    for px in buf.make_mut_bytes().chunks_exact_mut(4) {
        px.copy_from_slice(&[24, 28, 36, 255]); // R,G,B,A
    }
    slint::Image::from_rgba8(buf)
}

/// Default/empty track shown before playback starts.
pub fn empty_track() -> crate::TrackInfo {
    crate::TrackInfo {
        id: -1,
        title: slint::SharedString::from("Breezer"),
        artist: slint::SharedString::from(""),
        album: slint::SharedString::from(""),
        cover: placeholder_cover(),
        duration: 0.0,
    }
}

/// Format seconds as m:ss (e.g. 83 → "1:23").
pub fn fmt_duration(secs: f32) -> String {
    let s = secs.max(0.0) as u64;
    let (m, r) = (s / 60, s % 60);
    format!("{m}:{r:02}")
}