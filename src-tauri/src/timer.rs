//! Background refresh timer.
//!
//! Wakes every 10 seconds to check for a planetary hour change and
//! fires an immediate tray refresh + system notification at the boundary.
//! A separate 30-second heartbeat keeps the tray tooltip up to date.

use tauri::{AppHandle, Manager};
use std::time::Duration;
use astrohour_core::hours::current_hour;
use crate::commands::SharedState;

pub fn start_refresh_timer(app: AppHandle) {
    let app_heartbeat = app.clone();

    // ── Hour-boundary watcher (10 s resolution) ───────────────────────────────
    std::thread::spawn(move || {
        let mut last_hour_index: Option<u8> = None;

        loop {
            std::thread::sleep(Duration::from_secs(10));

            let hour = {
                let state = app.state::<SharedState>();
                let s = state.lock().unwrap();
                current_hour(s.location)
            };

            let current_index = Some(hour.index);

            let hour_changed = current_index.is_some()
                && last_hour_index.is_some()
                && current_index != last_hour_index;

            if hour_changed {
                // Refresh tray icon + tooltip immediately at the boundary
                crate::tray::refresh_tray(&app);
                // Send a system notification
                send_hour_notification(&app, &hour);
            }

            last_hour_index = current_index;
        }
    });

    // ── 30-second heartbeat — keeps tooltip/title current ─────────────────────
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        crate::tray::refresh_tray(&app_heartbeat);
    });
}

/// Fire a desktop notification announcing a new planetary hour.
fn send_hour_notification(app: &AppHandle, hour: &astrohour_core::types::PlanetaryHour) {
    use tauri_plugin_notification::NotificationExt;

    let glyph      = hour.planet.symbol();           // ♃  ♄  ☉  etc.
    let name       = hour.planet.name();             // Jupiter
    let start_str  = hour.start.format("%H:%M").to_string();
    let end_str    = hour.end.format("%H:%M").to_string();
    let day_night  = if hour.is_day { "Day" } else { "Night" };
    let seq        = hour.sequence;                  // 1-12

    let title = format!("{glyph} {name} Hour");
    let body  = format!("{day_night} hour {seq} · {start_str} – {end_str}");

    let result = app
        .notification()
        .builder()
        .title(&title)
        .body(&body)
        .show();

    if let Err(e) = result {
        eprintln!("[notify] Failed to send hour notification: {:?}", e);
    }
}
