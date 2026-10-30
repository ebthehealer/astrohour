//! AstroHour Tauri application backend.

mod commands;
mod db;
mod donation;
mod sound;
mod tray;
mod timer;


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_geolocation::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(db::DB_URL, db::migrations())
                .build()
        )
        .manage(std::sync::Mutex::new(commands::AppState::default()))
        .setup(|app| {
            tray::setup_tray(app)?;
            let app_handle = app.handle().clone();
            timer::start_refresh_timer(app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Prefs
            commands::init_prefs,
            commands::save_prefs,
            // Location
            commands::get_location,
            commands::set_location,
            // Autostart
            commands::get_autostart,
            commands::set_autostart,
            // Astronomical
            commands::get_current_hour,
            commands::get_day_schedule,
            commands::get_status_text,
            commands::get_moon_phase,
            // Display
            commands::get_display_mode,
            commands::set_display_mode,
            commands::set_tray_icon_color,
            commands::get_notifications_enabled,
            commands::set_notifications_enabled,
            commands::get_clock_format,
            commands::set_clock_format,
            commands::set_zodiac,
            // Reminders & sounds
            commands::get_reminder_minutes,
            commands::set_reminder_minutes,
            commands::get_hour_sound,
            commands::set_hour_sound,
            commands::get_reminder_sound,
            commands::set_reminder_sound,
            commands::preview_sound,
            // Notes helpers
            commands::make_planet_key,
            commands::today_string,
            // Donation reminders
            donation::dismiss_donation,
            donation::record_donation_click,
            #[cfg(debug_assertions)]
            donation::preview_donation,
        ])
        // Hide to tray on window close instead of quitting
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running AstroHour");
}
