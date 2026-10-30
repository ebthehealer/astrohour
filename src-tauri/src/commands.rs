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
use chrono::{DateTime, NaiveDate, Utc, FixedOffset, Local};

/// "Now" in the user's local timezone.
///
/// Prefers the UTC offset the frontend reported, because that is the same
/// value the UI uses to lay out the day — keeping the tray, the notifications
/// and the window in agreement. Falls back to the system timezone before the
/// frontend has had a chance to report one (e.g. during tray setup).
pub(crate) fn local_now(utc_offset_mins: Option<i32>) -> DateTime<FixedOffset> {
    match utc_offset_mins {
        Some(mins) => {
            let tz = FixedOffset::east_opt(mins * 60)
                .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
            Utc::now().with_timezone(&tz)
        }
        None => Local::now().fixed_offset(),
    }
}

/// Returns (current_hour, astrological_date) using the user's UTC offset.
/// Mirrors the Go apothecary logic: if before today's sunrise, use yesterday.
pub(crate) fn current_hour_and_date(
    loc: Location,
    utc_offset_mins: Option<i32>,
) -> (PlanetaryHour, NaiveDate) {
    let now_fixed = local_now(utc_offset_mins);
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
    /// Minutes-before-the-end-of-an-hour at which to fire a reminder.
    /// Empty means reminders are off. Values are drawn from `REMINDER_CHOICES`.
    pub reminder_minutes: Vec<u32>,
    /// Sound played with an hour-change notification. See `sound::SOUND_NAMES`.
    pub hour_sound: String,
    /// Sound played with a reminder notification. See `sound::SOUND_NAMES`.
    pub reminder_sound: String,
    /// UTC offset in minutes supplied by the frontend, so background threads
    /// can derive the astrological date the same way the UI does. `None` until
    /// the frontend has called `get_current_hour` at least once.
    pub utc_offset_mins: Option<i32>,
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
            // Opt-in: never start firing reminders the user didn't ask for.
            reminder_minutes: Vec::new(),
            hour_sound: "chime".to_string(),
            reminder_sound: "chime".to_string(),
            utc_offset_mins: None,
        }
    }
}

/// The reminder lead times the UI offers, in minutes.
pub const REMINDER_CHOICES: &[u32] = &[5, 10, 15, 20, 30];

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
    if let Some(arr) = store.get("reminder_minutes").and_then(|v| v.as_array().cloned()) {
        s.reminder_minutes = sanitize_reminders(
            arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect(),
        );
    }
    if let Some(v) = store.get("hour_sound").and_then(|v| v.as_str().map(String::from)) {
        if crate::sound::is_valid_sound(&v) {
            s.hour_sound = v;
        }
    }
    if let Some(v) = store.get("reminder_sound").and_then(|v| v.as_str().map(String::from)) {
        if crate::sound::is_valid_sound(&v) {
            s.reminder_sound = v;
        }
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
        tray_icon_color:       s.tray_icon_color.clone(),
        notifications_enabled: s.notifications_enabled,
        clock_format:          s.clock_format.clone(),
        reminder_minutes:      s.reminder_minutes.clone(),
        hour_sound:            s.hour_sound.clone(),
        reminder_sound:        s.reminder_sound.clone(),
        sound_playback:        crate::sound::available(),
    })
}

/// Keep only the lead times we actually offer, deduplicated and ordered.
///
/// Values arriving from the store or IPC are untrusted — an out-of-range entry
/// would otherwise fire a reminder at a nonsensical time.
fn sanitize_reminders(values: Vec<u32>) -> Vec<u32> {
    let mut out: Vec<u32> = REMINDER_CHOICES
        .iter()
        .copied()
        .filter(|c| values.contains(c))
        .collect();
    out.dedup();
    out
}

#[derive(serde::Serialize)]
pub struct InitPrefsResult {
    pub location_set:            bool,
    pub lat:                     f64,
    pub lon:                     f64,
    pub display_mode:            String,
    pub tray_icon_color:         String,
    pub notifications_enabled:   bool,
    pub clock_format:            String,
    pub reminder_minutes:        Vec<u32>,
    pub hour_sound:              String,
    pub reminder_sound:          String,
    /// false when the host has no audio player on PATH; the UI greys out the
    /// sound pickers in that case.
    pub sound_playback:          bool,
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
    store.set(
        "reminder_minutes",
        serde_json::Value::Array(
            s.reminder_minutes
                .iter()
                .map(|m| serde_json::Value::from(*m))
                .collect(),
        ),
    );
    store.set("hour_sound", s.hour_sound.clone());
    store.set("reminder_sound", s.reminder_sound.clone());
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

// ── Reminders & sounds ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_reminder_minutes(state: State<'_, SharedState>) -> Vec<u32> {
    state.lock().unwrap().reminder_minutes.clone()
}

#[tauri::command]
pub fn set_reminder_minutes(
    state: State<'_, SharedState>,
    minutes: Vec<u32>,
) -> Result<(), String> {
    if let Some(bad) = minutes.iter().find(|m| !REMINDER_CHOICES.contains(m)) {
        return Err(format!(
            "unsupported reminder lead time {bad}: expected one of {REMINDER_CHOICES:?}"
        ));
    }
    state.lock().unwrap().reminder_minutes = sanitize_reminders(minutes);
    Ok(())
}

#[tauri::command]
pub fn get_hour_sound(state: State<'_, SharedState>) -> String {
    state.lock().unwrap().hour_sound.clone()
}

#[tauri::command]
pub fn set_hour_sound(state: State<'_, SharedState>, sound: String) -> Result<(), String> {
    if !crate::sound::is_valid_sound(&sound) {
        return Err(format!("unknown sound {sound:?}"));
    }
    state.lock().unwrap().hour_sound = sound;
    Ok(())
}

#[tauri::command]
pub fn get_reminder_sound(state: State<'_, SharedState>) -> String {
    state.lock().unwrap().reminder_sound.clone()
}

#[tauri::command]
pub fn set_reminder_sound(state: State<'_, SharedState>, sound: String) -> Result<(), String> {
    if !crate::sound::is_valid_sound(&sound) {
        return Err(format!("unknown sound {sound:?}"));
    }
    state.lock().unwrap().reminder_sound = sound;
    Ok(())
}

/// Preview a sound from the settings UI.
#[tauri::command]
pub fn preview_sound(sound: String) -> Result<(), String> {
    if !crate::sound::is_valid_sound(&sound) {
        return Err(format!("unknown sound {sound:?}"));
    }
    if sound != "none" && sound != "system" && !crate::sound::available() {
        return Err(
            "No audio player found on this system (looked for pw-play, paplay, \
             canberra-gtk-play, aplay, ffplay)."
                .to_string(),
        );
    }
    crate::sound::play(&sound);
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
pub fn get_current_hour(
    state: State<SharedState>,
    utc_offset_mins: Option<i32>,
) -> PlanetaryHour {
    let loc = {
        let mut s = state.lock().unwrap();
        // Remember the offset so the background timer can derive the same
        // astrological date the UI is showing.
        s.utc_offset_mins = utc_offset_mins;
        s.location
    };
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
