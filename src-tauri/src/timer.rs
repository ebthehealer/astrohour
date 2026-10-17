//! Background refresh timer.
//!
//! Wakes every 30 seconds to emit `astro:tick` and refresh the tray.
//! Additionally detects planetary hour changes and fires an immediate
//! refresh at the boundary so the tray icon updates right on the dot.

use tauri::{AppHandle, Manager};
use std::time::Duration;
use astrohour_core::hours::current_hour;
use crate::commands::SharedState;

pub fn start_refresh_timer(app: AppHandle) {
    let app_heartbeat = app.clone();

    std::thread::spawn(move || {
        let mut last_hour_index: Option<u8> = None;

        loop {
            std::thread::sleep(Duration::from_secs(10));

            // Check if the planetary hour has changed
            let current_index: Option<u8> = {
                let state = app.state::<SharedState>();
                let s = state.lock().unwrap();
                Some(current_hour(s.location).index)
            };

            let hour_changed = current_index.is_some()
                && last_hour_index.is_some()
                && current_index != last_hour_index;

            if hour_changed {
                // Immediate refresh at the boundary
                crate::tray::refresh_tray(&app);
            }

            last_hour_index = current_index;

            // Always refresh every 30 seconds (counter: 3 × 10s)
            // We use a separate counter so boundary refresh doesn't reset the clock
        }
    });

    // Separate 30s heartbeat for regular tick + tray title update
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        crate::tray::refresh_tray(&app_heartbeat);
    });
}
