//! Deezer stream decryption.
//!
//! Deezer free-tier streams are AES-128-encrypted. The historical scheme derives
//! the AES key from the account's token and the track's `md5_origin`
//! (`md5(md5_origin + license_token)`), decrypting CBC chunks without padding.
//!
//! The whole scheme lives **here**, behind the [`StreamDecryptor`] trait, so it
//! can be audited independently and updated in one place if Deezer changes it.
//! v0.2 wires it into `Engine`.

use aes::cipher::{BlockDecryptMut, KeyIvInit};
use aes::Aes128;

/// AES-128-CBC with a zero IV (Deezer CDN stream chunks, legacy scheme).
type Aes128CbcDec = cbc::Decryptor<Aes128>;

// ---------------------------------------------------------------------------
// Blowfish-CBC "stripe" decryption (media.get_url / BF_CBC_STRIPE streams)
// — exactly the scheme of the tui-dzr client.
// ---------------------------------------------------------------------------

/// Deezer's Blowfish secret used to derive the key.
const DEEZER_SECRET: &str = "g4el58wc0zvf9na1";
/// Streaming chunks are 2048 bytes; every third chunk is encrypted.
const CHUNK_SIZE: usize = 2048;
const BLOWFISH_BLOCK_SIZE: usize = 8;
const DEEZER_BLOWFISH_IV: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

/// Derive the 16-byte Blowfish key from a numeric track id.
pub fn derive_blowfish_key(track_id: &str) -> [u8; 16] {
    let id_md5 = md5_hex(track_id.as_bytes());
    let id_md5_bytes = id_md5.as_bytes();
    let secret_bytes = DEEZER_SECRET.as_bytes();
    let mut key = [0u8; 16];
    for i in 0..16 {
        key[i] = id_md5_bytes[i] ^ id_md5_bytes[i + 16] ^ secret_bytes[i];
    }
    key
}

/// Decrypt a full Deezer encrypted stream in place (every 3rd chunk, CBC, no padding).
pub fn decrypt_audio_stream(
    track_id: &str,
    encrypted: &[u8],
) -> std::result::Result<Vec<u8>, DecryptError> {
    let key = derive_blowfish_key(track_id);
    decrypt_audio_stream_with_key(&key, encrypted)
}

pub fn decrypt_audio_stream_with_key(
    key: &[u8],
    encrypted: &[u8],
) -> std::result::Result<Vec<u8>, DecryptError> {
    let mut out = Vec::with_capacity(encrypted.len());
    for (chunk_index, chunk) in encrypted.chunks(CHUNK_SIZE).enumerate() {
        let mut decrypted = chunk.to_vec();
        decrypt_chunk_in_place_with_key(key, chunk_index, &mut decrypted)?;
        out.extend_from_slice(&decrypted);
    }
    Ok(out)
}

fn decrypt_chunk_in_place_with_key(
    key: &[u8],
    chunk_index: usize,
    chunk: &mut [u8],
) -> std::result::Result<(), DecryptError> {
    if chunk_index % 3 != 0 {
        return Ok(());
    }
    let decryptable_len = chunk.len() - (chunk.len() % BLOWFISH_BLOCK_SIZE);
    if decryptable_len == 0 {
        return Ok(());
    }
    use blowfish::cipher::{block_padding::NoPadding, BlockDecryptMut};
    let prefix = &mut chunk[..decryptable_len];
    let cipher = cbc::Decryptor::<blowfish::Blowfish>::new_from_slices(key, &DEEZER_BLOWFISH_IV)
        .map_err(|_| DecryptError::InvalidKey)?;
    cipher
        .decrypt_padded_mut::<NoPadding>(prefix)
        .map_err(|_| DecryptError::Io("blowfish decryption failed"))?;
    Ok(())
}

fn md5_hex(bytes: &[u8]) -> String {
    use md5::Digest;
    let digest = md5::Md5::digest(bytes);
    let mut out = String::with_capacity(32);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

// ---------------------------------------------------------------------------
// Trait & legacy AES decryptor
// ---------------------------------------------------------------------------

/// One unit of decryption logic.
pub trait StreamDecryptor: Send {
    /// Decrypt a full payload for `track_id`.
    fn decrypt(
        &self,
        track_id: u64,
        ciphertext: &[u8],
    ) -> std::result::Result<Vec<u8>, DecryptError>;
}

/// Block decryptor for the historical `md5_origin` scheme.
///
/// ```text
/// key = md5( md5_origin + license_token )
/// mode = AES-128-CBC, IV = 0x00 * 16
/// ```
/// Note: the exact key material is verified against a live account in v0.2 —
/// treat this as the pinned, isolated integration point.
pub struct Md5OriginDecryptor {
    key: [u8; 16],
    iv: [u8; 16],
}

impl Md5OriginDecryptor {
    /// Build from the pieces returned by the Deezer API.
    pub fn new(md5_origin: &str, license_token: &str) -> Self {
        let material = format!("{md5_origin}{license_token}");
        let digest = md5_digest(material.as_bytes());
        Self {
            key: digest,
            iv: [0u8; 16],
        }
    }
}

impl StreamDecryptor for Md5OriginDecryptor {
    fn decrypt(
        &self,
        _track_id: u64,
        ciphertext: &[u8],
    ) -> std::result::Result<Vec<u8>, DecryptError> {
        let mut cipher = Aes128CbcDec::new_from_slices(&self.key, &self.iv)
            .map_err(|_| DecryptError::InvalidKey)?;

        // Deezer chunks are whole AES blocks (16 bytes) with no padding; a
        // partial trailing block is passed through unchanged.
        let full_blocks = ciphertext.len() - (ciphertext.len() % 16);
        let mut out = ciphertext[..full_blocks].to_vec();

        use aes::cipher::generic_array::GenericArray;
        for chunk in out.chunks_exact_mut(16) {
            cipher.decrypt_block_mut(GenericArray::from_mut_slice(chunk));
        }

        if full_blocks < ciphertext.len() {
            out.extend_from_slice(&ciphertext[full_blocks..]);
        }
        Ok(out)
    }
}

/// MD5 digest (used for the key derivation).
fn md5_digest(bytes: &[u8]) -> [u8; 16] {
    use md5::Digest;
    let mut hasher = md5::Md5::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut key = [0u8; 16];
    key.copy_from_slice(&out[..16]);
    key
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecryptError {
    InvalidKey,
    Io(&'static str),
}

impl std::fmt::Display for DecryptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecryptError::InvalidKey => write!(f, "invalid AES key"),
            DecryptError::Io(m) => write!(f, "I/O during decryption: {m}"),
        }
    }
}

impl std::error::Error for DecryptError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The key derivation must be stable: `md5("abcxyz")`.
    #[test]
    fn key_derivation() {
        use md5::Digest;
        let d = Md5OriginDecryptor::new("abc", "xyz");
        let mut hasher = md5::Md5::new();
        hasher.update(b"abcxyz");
        let expect = hex::encode(hasher.finalize());
        assert_eq!(hex::encode(d.key), expect);
    }

    /// AES-CBC roundtrip with a zero IV.
    #[test]
    fn cbc_roundtrip() {
        use aes::cipher::generic_array::GenericArray;
        use aes::cipher::{BlockEncryptMut, KeyIvInit};
        use aes::Aes128;
        type Aes128CbcEnc = cbc::Encryptor<Aes128>;

        let d = Md5OriginDecryptor::new("keymaterial", "licensetokenvalue");
        let msg: Vec<u8> = (0u8..48).collect(); // exactly 3 AES blocks

        let mut enc = msg.clone();
        let mut cipher = Aes128CbcEnc::new_from_slices(&d.key, &[0u8; 16]).unwrap();
        for chunk in enc.chunks_exact_mut(16) {
            cipher.encrypt_block_mut(GenericArray::from_mut_slice(chunk));
        }

        let out = d.decrypt(1, &enc).unwrap();
        assert_eq!(out, msg);
    }

    /// Reference vector from the tui-dzr client: the key derivation must match.
    #[test]
    fn blowfish_key_stable() {
        let key = derive_blowfish_key("3135556");
        assert_eq!(hex::encode(key), "6c6c666b39662c37652575603c643439");
    }

    /// Only every 3rd 2048-byte chunk is encrypted; decryption restores data.
    #[test]
    fn blowfish_decrypts_only_third_chunks() {
        use blowfish::cipher::{block_padding::NoPadding, BlockEncryptMut};
        use cbc::Encryptor as BlowfishCbcEnc;

        let key = derive_blowfish_key("42");
        let plaintext = vec![0x5a; CHUNK_SIZE * 4];
        let mut encrypted = plaintext.clone();
        for chunk_index in [0usize, 3usize] {
            let start = chunk_index * CHUNK_SIZE;
            let chunk = &mut encrypted[start..start + CHUNK_SIZE];
            let mut cipher =
                BlowfishCbcEnc::<blowfish::Blowfish>::new_from_slices(&key, &DEEZER_BLOWFISH_IV)
                    .unwrap();
            cipher
                .encrypt_padded_mut::<NoPadding>(chunk, chunk.len())
                .unwrap();
        }
        let decrypted = decrypt_audio_stream_with_key(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
