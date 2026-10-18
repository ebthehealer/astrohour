//! Tauri IPC commands — wiring astrohour-core into the frontend.
//!
//! App state (location + display mode) lives in SharedState (Mutex).
//! On startup, init_prefs loads persisted values from tauri-plugin-store.
//! On change, the frontend calls save_prefs to persist them.

use astrohour_core::{
    Location, DisplayMode,
    types::{DaySchedule, MoonPhase, PlanetaryHour},
    hours::current_hour,
    lunar::moon_phase,
    display::status_bar_text,
    day_schedule,
};
use chrono::{NaiveDate, Utc};
use tauri::State;
use std::sync::Mutex;
use crate::license::sha256_hex;

// ── App state ──────────────────────────────────────────────────────────────────

pub struct AppState {
    pub location: Location,
    pub display_mode: DisplayMode,
    /// true once geolocation has succeeded or user has entered coords manually
    pub location_set: bool,
    /// Tray icon color: "white", "gold", or a hex string like "#ff8800"
    pub tray_icon_color: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            location: Location { lat: 40.7128, lon: -74.0060 },
            display_mode: DisplayMode::Short,
            location_set: false,
            tray_icon_color: "auto".to_string(),
        }
    }
}

pub type SharedState = Mutex<AppState>;

// ── Prefs persistence ──────────────────────────────────────────────────────────

/// Load saved prefs from tauri-plugin-store into AppState.
/// Called once from setup (via the frontend's onMount → init_prefs command).
#[tauri::command]
pub fn init_prefs(
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<InitPrefsResult, String> {
    use tauri_plugin_store::StoreExt;

    let store = app.store("prefs.json").map_err(|e| e.to_string())?;

    let lat = store.get("lat").and_then(|v| v.as_f64());
    let lon = store.get("lon").and_then(|v| v.as_f64());
    let mode_str = store
        .get("display_mode")
        .and_then(|v| v.as_str().map(String::from));

    let mut s = state.lock().unwrap();

    if let (Some(lat), Some(lon)) = (lat, lon) {
        s.location = Location { lat, lon };
        s.location_set = true;
    }

    if let Some(m) = &mode_str {
        s.display_mode = match m.as_str() {
            "IconOnly" => DisplayMode::IconOnly,
            "Full"     => DisplayMode::Full,
            _          => DisplayMode::Short,
        };
    }

    if let Some(color) = store.get("tray_icon_color").and_then(|v| v.as_str().map(String::from)) {
        s.tray_icon_color = color;
    }

    // Generate device secret if not already present.
    // Used as the stronghold vault password — stable per-install, invisible to user.
    if store.get("device_secret").is_none() {
        let secret = generate_device_secret();
        store.set("device_secret", serde_json::Value::String(secret));
        store.save().map_err(|e| e.to_string())?;
    }
    let device_secret = store
        .get("device_secret")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(generate_device_secret);

    Ok(InitPrefsResult {
        location_set: s.location_set,
        lat: s.location.lat,
        lon: s.location.lon,
        display_mode: match s.display_mode {
            DisplayMode::IconOnly => "IconOnly".into(),
            DisplayMode::Short    => "Short".into(),
            DisplayMode::Full     => "Full".into(),
        },
        device_secret,
        tray_icon_color: s.tray_icon_color.clone(),
    })
}

/// Generate a 64-char hex device secret from timestamp + PID mixed via SHA-256.
/// Not a cryptographic secret — just needs to be stable and unique per install.
fn generate_device_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts  = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let pid = std::process::id();
    sha256_hex(format!("astrohour-device-{ts}-{pid}").as_bytes())
}

#[derive(serde::Serialize)]
pub struct InitPrefsResult {
    pub location_set:     bool,
    pub lat:              f64,
    pub lon:              f64,
    pub display_mode:     String,
    pub device_secret:    String,
    pub tray_icon_color:  String,
}

/// Persist current prefs to store.
#[tauri::command]
pub fn save_prefs(
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;

    let store = app.store("prefs.json").map_err(|e| e.to_string())?;
    let s = state.lock().unwrap();

    store.set("lat", s.location.lat);
    store.set("lon", s.location.lon);
    store.set("display_mode", match s.display_mode {
        DisplayMode::IconOnly => "IconOnly",
        DisplayMode::Short    => "Short",
        DisplayMode::Full     => "Full",
    });
    store.set("tray_icon_color", s.tray_icon_color.clone());
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_tray_icon_color(
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
    color: String,
) -> Result<(), String> {
    state.lock().unwrap().tray_icon_color = color;
    crate::tray::refresh_tray(&app);
    Ok(())
}

// ── Location ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_location(
    state: State<SharedState>,
    lat: f64,
    lon: f64,
) -> Result<(), String> {
    if !(-90.0..=90.0).contains(&lat)  { return Err("latitude out of range".into()); }
    if !(-180.0..=180.0).contains(&lon) { return Err("longitude out of range".into()); }
    let mut s = state.lock().unwrap();
    s.location = Location { lat, lon };
    s.location_set = true;
    Ok(())
}

#[tauri::command]
pub fn get_location(state: State<SharedState>) -> LocationResult {
    let s = state.lock().unwrap();
    LocationResult { lat: s.location.lat, lon: s.location.lon, location_set: s.location_set }
}

#[derive(serde::Serialize)]
pub struct LocationResult {
    pub lat: f64,
    pub lon: f64,
    pub location_set: bool,
}

// ── Autostart ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

// ── Astronomical queries ───────────────────────────────────────────────────────

#[tauri::command]
pub fn get_current_hour(state: State<SharedState>) -> PlanetaryHour {
    let loc = state.lock().unwrap().location;
    current_hour(loc)
}

#[tauri::command]
pub fn get_day_schedule(
    state: State<SharedState>,
    date_str: Option<String>,
) -> Result<DaySchedule, String> {
    use chrono::Local;
    let loc = state.lock().unwrap().location;
    let date = match date_str {
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map_err(|e| format!("invalid date: {e}"))?,
        // Use the same date as current_hour(): local time, and if we're
        // before today's sunrise use yesterday so the schedule matches.
        None => {
            let cur = current_hour(loc);
            cur.start.with_timezone(&Local).date_naive()
        }
    };
    Ok(day_schedule(date, loc))
}

#[tauri::command]
pub fn get_status_text(state: State<SharedState>) -> String {
    let s = state.lock().unwrap();
    status_bar_text(s.location, s.display_mode)
}

#[tauri::command]
pub fn get_moon_phase() -> MoonPhase {
    moon_phase(Utc::now())
}

// ── Display mode ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_display_mode(state: State<SharedState>, mode: String) -> Result<(), String> {
    let m = match mode.as_str() {
        "IconOnly" => DisplayMode::IconOnly,
        "Short"    => DisplayMode::Short,
        "Full"     => DisplayMode::Full,
        other      => return Err(format!("unknown display mode: {other}")),
    };
    state.lock().unwrap().display_mode = m;
    Ok(())
}

#[tauri::command]
pub fn get_display_mode(state: State<SharedState>) -> String {
    match state.lock().unwrap().display_mode {
        DisplayMode::IconOnly => "IconOnly".into(),
        DisplayMode::Short    => "Short".into(),
        DisplayMode::Full     => "Full".into(),
    }
}

// ── Phase 2: Notes commands ─────────────────────────────────────────────────────────
//
// Notes are stored/queried via tauri-plugin-sql from the frontend.
// These commands are thin helpers for operations that need Rust-side
// logic (e.g. planet_key generation from a date + hour index).

/// Generate a planet_key string for a given date and hour index.
/// Format: "YYYY-MM-DD:N"
/// The frontend uses this to build keys before calling the SQL plugin directly.
#[tauri::command]
pub fn make_planet_key(date: String, hour_index: usize) -> Result<String, String> {
    // Validate date format
    NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| format!("invalid date: {e}"))?;
    if hour_index > 23 {
        return Err(format!("hour_index {hour_index} out of range (0-23)"));
    }
    Ok(format!("{date}:{hour_index}"))
}

/// Return today's date string in YYYY-MM-DD (UTC).
/// Convenience for the frontend so it doesn't have to format dates itself.
#[tauri::command]
pub fn today_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

// ── Phase 2: License commands ───────────────────────────────────────────────────────

use crate::license::{validate_key, LicenseStatus, SharedLicense};

/// Return the current license status for the frontend.
/// Called on startup and after key entry.
///
/// On Linux all features are free — always returns Active with a
/// special "linux-free" key hash so the frontend knows to show
/// a donation prompt instead of a license UI.
#[tauri::command]
pub fn get_license_status(license: State<SharedLicense>) -> LicenseStatus {
    #[cfg(target_os = "linux")]
    {
        return LicenseStatus::Active {
            key_hash:     "linux-free".into(),
            activated_at: "linux-free".into(),
        };
    }
    #[cfg(not(target_os = "linux"))]
    license.lock().unwrap().clone()
}

/// Validate and activate a license key.
///
/// On success:
///   - Updates SharedLicense state to Active
///   - The frontend is responsible for persisting the raw key via
///     `@tauri-apps/plugin-stronghold` (JS side) and writing the hash
///     to SQLite.
///
/// Returns `Ok(key_hash)` on success, `Err(message)` on failure.
#[tauri::command]
pub fn activate_license(
    license: State<SharedLicense>,
    raw_key: String,
) -> Result<String, String> {
    let key_hash = validate_key(&raw_key).map_err(|e| e.to_string())?;
    let activated_at = Utc::now().to_rfc3339();
    *license.lock().unwrap() = LicenseStatus::Active {
        key_hash: key_hash.clone(),
        activated_at,
    };
    Ok(key_hash)
}

/// Deactivate the current license (for testing / support).
#[tauri::command]
pub fn deactivate_license(license: State<SharedLicense>) {
    *license.lock().unwrap() = LicenseStatus::None;
}

/// Load license status from SQLite + stronghold on startup.
/// Called by the frontend after DB is ready. Accepts the stored key_hash
/// and activated_at from the DB row; the raw key is held in stronghold
/// on the JS side.
#[tauri::command]
pub fn restore_license(
    license: State<SharedLicense>,
    key_hash: Option<String>,
    activated_at: Option<String>,
) {
    let status = match (key_hash, activated_at) {
        (Some(hash), Some(at)) if !hash.is_empty() => {
            LicenseStatus::Active { key_hash: hash, activated_at: at }
        }
        _ => LicenseStatus::None,
    };
    *license.lock().unwrap() = status;
}
