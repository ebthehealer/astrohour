//! AstroHour Tauri application backend.

mod commands;
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
        .manage(std::sync::Mutex::new(commands::AppState::default()))
        .setup(|app| {
            tray::setup_tray(app)?;
            let app_handle = app.handle().clone();
            timer::start_refresh_timer(app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_current_hour,
            commands::get_day_schedule,
            commands::get_status_text,
            commands::get_moon_phase,
            commands::set_location,
            commands::get_location,
            commands::set_display_mode,
            commands::get_display_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AstroHour");
}
