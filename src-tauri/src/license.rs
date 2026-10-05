//! License key validation and storage.
//!
//! ## Key format
//! `ASTROHOUR-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX`
//!
//! The 4 × 8-char segments (32 chars total, base32 RFC4648) decode to 20 bytes:
//!   bytes  0..4   — version byte (0x01) + 3 reserved bytes
//!   bytes  4..20  — truncated to fit; the Cloudflare Worker uses the full
//!                   64-byte Ed25519 signature split across an extended format
//!
//! Real key encoding produced by the Cloudflare Worker:
//!   payload = version(1) || reserved(3) || epoch_days(2)  [6 bytes]
//!   sig     = Ed25519_sign(private_key, payload)          [64 bytes]
//!   raw     = payload || sig                              [70 bytes]
//!   encoded = base32(raw) padded to 8-char segments with dashes
//!
//! The full key is:
//!   "ASTROHOUR-" + base32(raw)[0..8] + "-" + ... (9 segments of 8)
//!   Total: "ASTROHOUR-" + 9*8 + 8*"-" = 10 + 72 + 8 = 90 chars
//!
//! For human entry we use a shorter prefix-verified format and
//! verify only the embedded signature bytes.

use ed25519_dalek::{Signature, VerifyingKey, ed25519::signature::Verifier};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use sha2::{Digest, Sha256};

// ── Embedded public key ─────────────────────────────────────────────────────
//
// Raw 32-byte Ed25519 public key (little-endian compressed Edwards point).
// The corresponding private key is held ONLY in the Cloudflare Worker.
// This public key is safe to embed — it cannot be used to forge signatures.
const PUBLIC_KEY: [u8; 32] = [
    0xd9, 0x97, 0x5d, 0x3f, 0x1c, 0x33, 0x05, 0xde,
    0x1f, 0x58, 0xb3, 0xe9, 0xaa, 0x7a, 0x30, 0x5f,
    0x56, 0x4d, 0x33, 0xd7, 0xbd, 0xb9, 0x7b, 0xad,
    0xd8, 0x8c, 0xc9, 0xd4, 0xe0, 0xc6, 0x31, 0xcb,
];

// ── Public types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LicenseStatus {
    None,
    Active { key_hash: String, activated_at: String },
    Invalid,
}

impl LicenseStatus {
    pub fn is_premium(&self) -> bool {
        matches!(self, LicenseStatus::Active { .. })
    }
}

// ── Validation errors ──────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    BadFormat,
    BadEncoding,
    BadSignature,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::BadFormat    => write!(f, "Invalid key format"),
            ValidationError::BadEncoding  => write!(f, "Invalid key encoding"),
            ValidationError::BadSignature => write!(f, "Invalid or expired license key"),
        }
    }
}

// ── Key validation ─────────────────────────────────────────────────────────────

/// Validate a license key string.
///
/// Key format: `ASTROHOUR-XXXXXXXX-XXXXXXXX-...-XXXXXXXX`
/// (prefix + 14 groups of 8 base32 chars = 112 chars encoded = 70 raw bytes)
///
/// Layout of decoded bytes (70 bytes):
///   [0]      version = 0x01
///   [1..4]   reserved (zeros)
///   [4..6]   epoch_days: days since 2025-01-01 (big-endian u16)
///   [6..70]  Ed25519 signature over bytes [0..6]
///
/// Returns `Ok(key_hash)` — hex SHA-256 of the normalised key — on success.
pub fn validate_key(raw_key: &str) -> Result<String, ValidationError> {
    let key = raw_key.trim().to_uppercase();

    // Parse: "ASTROHOUR-" + 14 groups of 8 base32 chars separated by "-"
    let rest = key.strip_prefix("ASTROHOUR-")
        .ok_or(ValidationError::BadFormat)?;

    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() != 14 {
        return Err(ValidationError::BadFormat);
    }
    for seg in &parts {
        if seg.len() != 8 || !seg.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(ValidationError::BadFormat);
        }
    }

    // Base32-decode the 112-char payload (14 × 8)
    let encoded: String = parts.concat();
    let decoded = base32_decode(&encoded).ok_or(ValidationError::BadEncoding)?;

    // Need exactly 6 bytes payload + 64 bytes signature = 70 bytes
    if decoded.len() < 70 {
        return Err(ValidationError::BadEncoding);
    }

    let payload   = &decoded[..6];
    let sig_bytes = &decoded[6..70];

    // Version check
    if payload[0] != 0x01 {
        return Err(ValidationError::BadFormat);
    }

    // Ed25519 signature verification
    let vk = VerifyingKey::from_bytes(&PUBLIC_KEY)
        .map_err(|_| ValidationError::BadSignature)?;

    let sig_arr: [u8; 64] = sig_bytes.try_into()
        .map_err(|_| ValidationError::BadEncoding)?;
    let sig = Signature::from_bytes(&sig_arr);

    vk.verify(payload, &sig)
        .map_err(|_| ValidationError::BadSignature)?;

    // SHA-256 of normalised key for storage/display
    let hash = sha256_hex(key.as_bytes());
    Ok(hash)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn sha256_hex(data: &[u8]) -> String {
    let bytes = Sha256::digest(data);
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// RFC 4648 base32 decoder (no padding required, uppercase input).
pub fn base32_decode(s: &str) -> Option<Vec<u8>> {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits: u64 = 0;
    let mut bits_left: u32 = 0;
    let mut out = Vec::new();
    for ch in s.chars() {
        let val = ALPHA.iter().position(|&b| b == ch as u8)? as u64;
        bits = (bits << 5) | val;
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            out.push((bits >> bits_left) as u8);
        }
    }
    Some(out)
}

/// RFC 4648 base32 encoder (no padding, uppercase).
pub fn base32_encode(data: &[u8]) -> String {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    let mut bits: u64 = 0;
    let mut bits_left: u32 = 0;
    for &byte in data {
        bits = (bits << 8) | byte as u64;
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            out.push(ALPHA[(bits >> bits_left) as usize & 0x1f] as char);
        }
    }
    if bits_left > 0 {
        out.push(ALPHA[((bits << (5 - bits_left)) & 0x1f) as usize] as char);
    }
    out
}

// ── Managed state ──────────────────────────────────────────────────────────────

use std::sync::Mutex;
pub type SharedLicense = Mutex<LicenseStatus>;

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use ed25519_dalek::VerifyingKey;
    use ed25519_dalek::ed25519::signature::Signer;

    /// Build a test key signed with the given signing key, and a matching
    /// verifying key so we can patch PUBLIC_KEY for the test.
    fn make_test_keypair_and_key() -> (VerifyingKey, String) {
        // Generate a fresh ephemeral keypair — never the real private key.
        // We override PUBLIC_KEY in validate_key_with() below.
        let seed: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ];
        let signing_key  = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        let mut payload = [0u8; 6];
        payload[0] = 0x01;

        let sig: ed25519_dalek::Signature = signing_key.sign(&payload);
        let mut raw = Vec::with_capacity(70);
        raw.extend_from_slice(&payload);
        raw.extend_from_slice(&sig.to_bytes());

        let encoded = base32_encode(&raw);
        let padded  = format!("{:A<112}", encoded);
        let groups: Vec<&str> = (0..14).map(|i| &padded[i*8..(i+1)*8]).collect();
        let key = format!("ASTROHOUR-{}", groups.join("-"));

        (verifying_key, key)
    }

    /// validate_key variant that accepts a custom public key — test use only.
    fn validate_key_with(raw_key: &str, pub_key: &[u8; 32]) -> Result<String, ValidationError> {
        let key  = raw_key.trim().to_uppercase();
        let rest = key.strip_prefix("ASTROHOUR-").ok_or(ValidationError::BadFormat)?;
        let parts: Vec<&str> = rest.split('-').collect();
        if parts.len() != 14 { return Err(ValidationError::BadFormat); }
        for seg in &parts {
            if seg.len() != 8 || !seg.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(ValidationError::BadFormat);
            }
        }
        let encoded = parts.concat();
        let decoded = base32_decode(&encoded).ok_or(ValidationError::BadEncoding)?;
        if decoded.len() < 70 { return Err(ValidationError::BadEncoding); }
        if decoded[0] != 0x01 { return Err(ValidationError::BadFormat); }
        let payload = &decoded[..6];
        let sig_bytes: [u8; 64] = decoded[6..70].try_into().map_err(|_| ValidationError::BadEncoding)?;
        let vk  = VerifyingKey::from_bytes(pub_key).map_err(|_| ValidationError::BadSignature)?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        use ed25519_dalek::Verifier;
        vk.verify(payload, &sig).map_err(|_| ValidationError::BadSignature)?;
        Ok(sha256_hex(key.as_bytes()))
    }

    #[test]
    fn base32_roundtrip() {
        let data = b"hello world test";
        let enc  = base32_encode(data);
        let dec  = base32_decode(&enc).unwrap();
        assert_eq!(&dec, data);
    }

    #[test]
    fn bad_format_rejected() {
        assert_eq!(validate_key("WRONG-FORMAT"), Err(ValidationError::BadFormat));
        assert_eq!(validate_key("ASTROHOUR-TOOSHORT"), Err(ValidationError::BadFormat));
    }

    #[test]
    fn valid_key_accepted() {
        // Uses an ephemeral test keypair — real private key never appears in source.
        let (vk, key) = make_test_keypair_and_key();
        let pub_bytes  = vk.to_bytes();
        assert!(
            validate_key_with(&key, &pub_bytes).is_ok(),
            "valid key should pass: {key}"
        );
    }
}
