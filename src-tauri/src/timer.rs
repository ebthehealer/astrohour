//! 30-second background refresh timer.
//!
//! Spawns a Tokio task that wakes every 30 seconds and calls
//! `tray::refresh_tray()` to update the status bar text and emit
//! the `astro:tick` event to any open windows.

use tauri::AppHandle;
use std::time::Duration;

pub fn start_refresh_timer(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        crate::tray::refresh_tray(&app);
    });
}
