//! Tauri IPC commands — wiring astrohour-core into the frontend.
//!
//! App state (location + display mode) lives in SharedState (Mutex).
//! On startup, init_prefs loads persisted values from tauri-plugin-store.
//! On change, the frontend calls save_prefs to persist them.

use astrohour_core::{
    Location, DisplayMode,
    types::{MoonPhase, PlanetaryHour},
    lunar::moon_phase,
    display::status_bar_text,
    day_schedule,
};
use chrono::{NaiveDate, Utc, FixedOffset};

/// Return today's date in the user's local timezone.
/// utc_offset_mins: e.g. -240 for EDT, -300 for EST, 60 for CET.
fn local_today(utc_offset_mins: Option<i32>) -> NaiveDate {
    let offset_secs = utc_offset_mins.unwrap_or(0) * 60;
    let tz = FixedOffset::east_opt(offset_secs).unwrap_or(FixedOffset::east_opt(0).unwrap());
    Utc::now().with_timezone(&tz).date_naive()
}

/// Returns (current_hour, astrological_date) using the user's UTC offset.
/// Mirrors the Go apothecary logic: if before today's sunrise, use yesterday.
fn current_hour_and_date(loc: Location, utc_offset_mins: Option<i32>) -> (PlanetaryHour, NaiveDate) {
    let offset_secs = utc_offset_mins.unwrap_or(0) * 60;
    let tz = FixedOffset::east_opt(offset_secs).unwrap_or(FixedOffset::east_opt(0).unwrap());
    let now_fixed = Utc::now().with_timezone(&tz);
    let today = now_fixed.date_naive();

    // Try today's hours first
    let hours = astrohour_core::hours::planetary_hours(today, loc);
    if let Some(h) = hours.iter().find(|h| now_fixed >= h.start && now_fixed < h.end) {
        return (h.clone(), today);
    }
    // Before today's sunrise: use yesterday's night hours
    let yesterday = today.pred_opt().unwrap_or(today);
    let prev = astrohour_core::hours::planetary_hours(yesterday, loc);
    if let Some(h) = prev.iter().find(|h| now_fixed >= h.start && now_fixed < h.end) {
        return (h.clone(), yesterday);
    }
    (hours.into_iter().next().expect("hours not empty"), today)
}

fn current_hour_local(loc: Location, utc_offset_mins: Option<i32>) -> PlanetaryHour {
    current_hour_and_date(loc, utc_offset_mins).0
}
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
    /// Whether to send a system notification on each hour change
    pub notifications_enabled: bool,
    /// "12" or "24"
    pub clock_format: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            location: Location { lat: 40.7128, lon: -74.0060 },
            display_mode: DisplayMode::Short,
            location_set: false,
            tray_icon_color: "auto".to_string(),
            notifications_enabled: true,
            clock_format: "12".to_string(),
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
    if let Some(v) = store.get("notifications_enabled").and_then(|v| v.as_bool()) {
        s.notifications_enabled = v;
    }
    if let Some(v) = store.get("clock_format").and_then(|v| v.as_str().map(String::from)) {
        s.clock_format = v;
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
        tray_icon_color:       s.tray_icon_color.clone(),
        notifications_enabled: s.notifications_enabled,
        clock_format:          s.clock_format.clone(),
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
    pub location_set:            bool,
    pub lat:                     f64,
    pub lon:                     f64,
    pub display_mode:            String,
    pub device_secret:           String,
    pub tray_icon_color:         String,
    pub notifications_enabled:   bool,
    pub clock_format:            String,
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
    store.set("notifications_enabled", s.notifications_enabled);
    store.set("clock_format", s.clock_format.clone());
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

#[tauri::command]
pub fn set_notifications_enabled(state: State<'_, SharedState>, enabled: bool) -> Result<(), String> {
    state.lock().unwrap().notifications_enabled = enabled;
    Ok(())
}

#[tauri::command]
pub fn get_notifications_enabled(state: State<'_, SharedState>) -> bool {
    state.lock().unwrap().notifications_enabled
}

#[tauri::command]
pub fn set_clock_format(state: State<'_, SharedState>, format: String) -> Result<(), String> {
    if format != "12" && format != "24" {
        return Err(format!("invalid clock format {:?}: must be '12' or '24'", format));
    }
    state.lock().unwrap().clock_format = format;
    Ok(())
}

#[tauri::command]
pub fn get_clock_format(state: State<'_, SharedState>) -> String {
    state.lock().unwrap().clock_format.clone()
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
pub fn get_current_hour(
    state: State<SharedState>,
    utc_offset_mins: Option<i32>,
) -> PlanetaryHour {
    let loc = state.lock().unwrap().location;
    current_hour_local(loc, utc_offset_mins)
}

/// Extended schedule returned to the frontend.
#[derive(serde::Serialize)]
pub struct DayScheduleResult {
    pub date:           String,
    pub location:       astrohour_core::types::Location,
    pub planetary_day:  String,
    pub sunrise:        String,
    pub sunset:         String,
    pub next_new_moon:  Option<String>,  // RFC3339 local
    pub next_full_moon: Option<String>,  // RFC3339 local
    pub hours:          Vec<astrohour_core::types::PlanetaryHour>,
    pub moon:           astrohour_core::types::MoonPhase,
}

#[tauri::command]
pub fn get_day_schedule(
    state: State<SharedState>,
    date_str: Option<String>,
    utc_offset_mins: Option<i32>,
) -> Result<DayScheduleResult, String> {
    use chrono::Datelike;
    use solunatus::prelude::{get_lunar_phases_for_month, LunarPhaseType};

    let loc = state.lock().unwrap().location;
    // When date_str is provided (travel date), use it directly.
    // When None, derive the astrological date from the UTC offset:
    // if we are before today's sunrise, the active schedule is yesterday's.
    let date = match date_str {
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map_err(|e| format!("invalid date: {e}"))?,
        None => {
            // Use the same date that current_hour_and_date resolved:
            // yesterday if before sunrise, today otherwise.
            current_hour_and_date(loc, utc_offset_mins).1
        }
    };

    let sched = day_schedule(date, loc);

    // Find next new moon and full moon from today, searching up to 3 months ahead
    // Use user's UTC offset so "today" is correct in their timezone
    let offset_secs = utc_offset_mins.unwrap_or(0) * 60;
    let user_tz = FixedOffset::east_opt(offset_secs).unwrap_or(FixedOffset::east_opt(0).unwrap());
    let now_fixed = Utc::now().with_timezone(&user_tz);
    let mut next_new:  Option<String> = None;
    let mut next_full: Option<String> = None;

    'outer: for month_offset in 0..3i32 {
        let search_date = now_fixed.date_naive()
            + chrono::Months::new(month_offset as u32);
        let phases = get_lunar_phases_for_month(search_date.year(), search_date.month())
            .map_err(|e| e.to_string())?;
        for p in phases {
            if p.datetime <= Utc::now() { continue; }
            let local_dt = p.datetime.with_timezone(&user_tz);
            match p.phase_type {
                LunarPhaseType::NewMoon  if next_new.is_none()  => {
                    next_new = Some(local_dt.to_rfc3339());
                }
                LunarPhaseType::FullMoon if next_full.is_none() => {
                    next_full = Some(local_dt.to_rfc3339());
                }
                _ => {}
            }
            if next_new.is_some() && next_full.is_some() { break 'outer; }
        }
    }

    Ok(DayScheduleResult {
        date:           sched.date.to_string(),
        location:       sched.location,
        planetary_day:  sched.planetary_day.name().to_string(),
        sunrise:        sched.sunrise.to_rfc3339(),
        sunset:         sched.sunset.to_rfc3339(),
        next_new_moon:  next_new,
        next_full_moon: next_full,
        hours:          sched.hours,
        moon:           sched.moon,
    })
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
pub fn get_license_status(_license: State<SharedLicense>) -> LicenseStatus {
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
