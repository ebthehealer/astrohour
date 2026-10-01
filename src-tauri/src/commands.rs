//! Tauri IPC commands — each corresponds to one astrohour-core function.
//!
//! Location is stored in tauri-plugin-store and passed to every calculation.
//! All return types are serializable (astrohour-core types all derive Serialize).

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

// ── App state ──────────────────────────────────────────────────────────────────────

/// Mutable app state held in Tauri's managed state.
pub struct AppState {
    pub location: Location,
    pub display_mode: DisplayMode,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            // Default: New York. Overwritten by geolocation or saved prefs on startup.
            location: Location { lat: 40.7128, lon: -74.0060 },
            display_mode: DisplayMode::Short,
        }
    }
}

pub type SharedState = Mutex<AppState>;

// ── Commands ────────────────────────────────────────────────────────────────────────

/// Returns the current planetary hour for the user's location.
#[tauri::command]
pub fn get_current_hour(state: State<SharedState>) -> PlanetaryHour {
    let loc = state.lock().unwrap().location;
    current_hour(loc)
}

/// Returns the full 24-hour schedule for today.
#[tauri::command]
pub fn get_day_schedule(
    state: State<SharedState>,
    date_str: Option<String>,  // ISO date "2025-01-09", None = today
) -> Result<DaySchedule, String> {
    let loc = state.lock().unwrap().location;
    let date = match date_str {
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map_err(|e| format!("invalid date: {e}"))?,
        None => Utc::now().date_naive(),
    };
    Ok(day_schedule(date, loc))
}

/// Returns the status bar text string for the current display mode.
#[tauri::command]
pub fn get_status_text(state: State<SharedState>) -> String {
    let s = state.lock().unwrap();
    status_bar_text(s.location, s.display_mode)
}

/// Returns current moon phase data.
#[tauri::command]
pub fn get_moon_phase() -> MoonPhase {
    moon_phase(Utc::now())
}

/// Save the user's location (called after geolocation or manual entry).
#[tauri::command]
pub fn set_location(state: State<SharedState>, lat: f64, lon: f64) -> Result<(), String> {
    if lat < -90.0 || lat > 90.0 { return Err("latitude out of range".into()); }
    if lon < -180.0 || lon > 180.0 { return Err("longitude out of range".into()); }
    state.lock().unwrap().location = Location { lat, lon };
    Ok(())
}

/// Get the current stored location.
#[tauri::command]
pub fn get_location(state: State<SharedState>) -> Location {
    state.lock().unwrap().location
}

/// Set the display mode (IconOnly / Short / Full).
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

/// Get the current display mode as a string.
#[tauri::command]
pub fn get_display_mode(state: State<SharedState>) -> String {
    match state.lock().unwrap().display_mode {
        DisplayMode::IconOnly => "IconOnly".into(),
        DisplayMode::Short    => "Short".into(),
        DisplayMode::Full     => "Full".into(),
    }
}
