//! System tray / menu bar setup.

use tauri::{
    App,
    Emitter,
    Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
};
use crate::commands::SharedState;

pub fn setup_tray(app: &mut App) -> tauri::Result<()> {
    let open_i = MenuItem::with_id(app, "open", "Open AstroHour", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit",           true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_i, &quit_i])?;

    let initial_text = initial_tray_text(app);

    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("AstroHour")
        .title(&initial_text)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = win.hide();
                    } else {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// Update the tray title and emit a tick event to any open windows.
pub fn refresh_tray(app: &tauri::AppHandle) {
    use astrohour_core::display::status_bar_text;

    let state = app.state::<SharedState>();
    let s = state.lock().unwrap();
    let text = status_bar_text(s.location, s.display_mode);
    drop(s);

    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(Some(&text));
        let _ = tray.set_tooltip(Some(&text));
    }

    let _ = app.emit("astro:tick", ());
}

fn initial_tray_text(app: &App) -> String {
    use astrohour_core::display::status_bar_text;
    let state = app.state::<SharedState>();
    let s = state.lock().unwrap();
    status_bar_text(s.location, s.display_mode)
}
