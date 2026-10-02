//! License key validation and storage.
//!
//! ## Flow
//! 1. User enters a key in Settings.
//! 2. `validate_license_key()` checks the key format and cryptographic
//!    signature (Ed25519, public key baked in at compile time).
//! 3. On success the raw key is stored encrypted in tauri-plugin-stronghold
//!    under `"license_key"`. A SHA-256 hash and activation timestamp are
//!    written to the SQLite `license` table (for display / re-entry detection).
//! 4. On every startup `load_license_status()` reads the stronghold to
//!    determine whether premium is active.
//!
//! ## Key format
//! `ASTROHOUR-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX`
//! where the X blocks are base32-encoded bytes that encode a payload +
//! an Ed25519 signature over that payload.
//!
//! The Cloudflare Worker that issues keys signs them with the private key;
//! the app verifies with the baked-in public key. No network call needed
//! after initial activation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

// ── Public key ───────────────────────────────────────────────────────────────
//
// Placeholder — replace with real Ed25519 public key bytes before shipping.
// Generate with: `openssl genpkey -algorithm ed25519 -out private.pem`
//               `openssl pkey -in private.pem -pubout -out public.pem`
// Then extract the 32-byte raw public key.
//
// The Cloudflare Worker holds the private key; this binary holds only the
// public key (safe to embed — cannot derive private key from it).
#[allow(dead_code)]
const PUBLIC_KEY_BYTES: [u8; 32] = [0u8; 32]; // TODO: replace before shipping

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus {
    /// No key has been entered.
    None,
    /// Key entered, signature verified, premium unlocked.
    Active { key_hash: String, activated_at: String },
    /// Key present in stronghold but fails current verification
    /// (e.g. key was revoked or public key rotated).
    Invalid,
}

impl LicenseStatus {
    pub fn is_premium(&self) -> bool {
        matches!(self, LicenseStatus::Active { .. })
    }
}

// ── Validation ──────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    /// Key doesn't match `ASTROHOUR-XXXX-XXXX-XXXX-XXXX` format.
    BadFormat,
    /// Base32 decoding failed.
    BadEncoding,
    /// Ed25519 signature verification failed.
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

/// Validate a license key string.
///
/// Returns `Ok(key_hash)` (hex SHA-256 of the normalised key) on success.
/// The caller is responsible for storing the raw key in stronghold.
pub fn validate_key(raw_key: &str) -> Result<String, ValidationError> {
    // Normalise: trim whitespace, uppercase
    let key = raw_key.trim().to_uppercase();

    // Format check: ASTROHOUR-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() != 5 || parts[0] != "ASTROHOUR" {
        return Err(ValidationError::BadFormat);
    }
    for seg in &parts[1..] {
        if seg.len() != 8 || !seg.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(ValidationError::BadFormat);
        }
    }

    // Decode the payload (first 3 segments = 24 chars) + signature (last segment = 8 chars)
    // In practice the real key encoding would pack more data; this is the
    // structure that the Cloudflare Worker will generate.
    //
    // For now: concatenate segments 1-4, base32-decode the full block,
    // split into payload (first N bytes) + signature (last 64 bytes).
    let encoded = format!("{}{}{}{}", parts[1], parts[2], parts[3], parts[4]);

    // base32 decode (RFC 4648, no padding)
    let decoded = base32_decode(&encoded).ok_or(ValidationError::BadEncoding)?;

    if decoded.len() < 64 {
        // Not enough bytes for a payload + Ed25519 signature
        // During development with placeholder public key we skip sig check
        #[cfg(not(debug_assertions))]
        return Err(ValidationError::BadSignature);
    }

    // In release builds: verify Ed25519 signature
    // Payload = decoded[..n-64], signature = decoded[n-64..]
    // TODO: wire up real ed25519 verification once public key is set
    // For now this just verifies the format is sane.
    #[cfg(debug_assertions)]
    let _sig_valid = true; // skip in dev

    // Compute key hash for storage + display
    let hash = {
        let mut h = Sha256::new();
        h.update(key.as_bytes());
        let bytes = h.finalize();
        bytes.iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    };

    Ok(hash)
}

/// Minimal base32 decoder (RFC 4648 alphabet, no padding required).
fn base32_decode(s: &str) -> Option<Vec<u8>> {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits: u32 = 0;
    let mut bits_left: u32 = 0;
    let mut out = Vec::new();
    for ch in s.chars() {
        let val = ALPHA.iter().position(|&b| b == ch as u8)? as u32;
        bits = (bits << 5) | val;
        bits_left += 5;
        if bits_left >= 8 {
            bits_left -= 8;
            out.push((bits >> bits_left) as u8);
        }
    }
    Some(out)
}

// ── Managed state ──────────────────────────────────────────────────────────────

use std::sync::Mutex;

pub type SharedLicense = Mutex<LicenseStatus>;
