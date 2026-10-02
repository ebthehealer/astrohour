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

// ── App state ──────────────────────────────────────────────────────────────────

pub struct AppState {
    pub location: Location,
    pub display_mode: DisplayMode,
    /// true once geolocation has succeeded or user has entered coords manually
    pub location_set: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            location: Location { lat: 40.7128, lon: -74.0060 },
            display_mode: DisplayMode::Short,
            location_set: false,
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

    Ok(InitPrefsResult {
        location_set: s.location_set,
        lat: s.location.lat,
        lon: s.location.lon,
        display_mode: match s.display_mode {
            DisplayMode::IconOnly => "IconOnly".into(),
            DisplayMode::Short    => "Short".into(),
            DisplayMode::Full     => "Full".into(),
        },
    })
}

#[derive(serde::Serialize)]
pub struct InitPrefsResult {
    pub location_set: bool,
    pub lat: f64,
    pub lon: f64,
    pub display_mode: String,
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
    store.save().map_err(|e| e.to_string())?;
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
    let loc = state.lock().unwrap().location;
    let date = match date_str {
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map_err(|e| format!("invalid date: {e}"))?,
        None => Utc::now().date_naive(),
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
