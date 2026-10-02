//! System tray / menu bar icon setup and management.
//!
//! ## Platform behaviour
//!
//! ### macOS
//! The window is frameless and positioned directly below the menu bar icon
//! using `tauri-plugin-positioner` (TrayBottomCenter). The tray title is set
//! to the status bar text so it appears as text in the menu bar.
//! NSWindow level is raised to float above other windows.
//!
//! ### Linux
//! AppIndicator tray with title where the DE supports it.
//! Window positioned at TrayBottomCenter.
//!
//! ### Windows
//! Standard notification-area icon. The icon PNG is redrawn every 30 seconds
//! by `win_icon::render()` using tiny-skia to show the current planet glyph.

use tauri::{
    App, Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_positioner::{Position, WindowExt as PositionerExt, on_tray_event};
use crate::commands::SharedState;

// ── Setup ─────────────────────────────────────────────────────────────────────

pub fn setup_tray(app: &mut App) -> tauri::Result<()> {
    let open_i = MenuItem::with_id(app, "open", "Open AstroHour", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit",           true, None::<&str>)?;
    let menu   = Menu::with_items(app, &[&open_i, &quit_i])?;

    let initial_text = initial_tray_text(app);

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("AstroHour")
        .title(&initial_text)   // shown as menu-bar text on macOS
        .on_tray_icon_event(|tray, event| {
            // Let positioner record tray rect for TrayBottomCenter
            let app = tray.app_handle();
            on_tray_event(app, &event);

            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left, ..
            } = event {
                toggle_window(app);
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        });

    // On Windows, use a rendered glyph icon; on macOS use template icon
    #[cfg(target_os = "windows")]
    if let Some(icon) = win_icon::render(&initial_text) {
        builder = builder.icon(icon);
    }

    #[cfg(target_os = "macos")]
    { builder = builder.icon_as_template(true); }

    builder.build(app)?;
    Ok(())
}

// ── Window toggle / show ──────────────────────────────────────────────────────────

fn toggle_window(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
    } else {
        show_window(app);
    }
}

fn show_window(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };

    // Snap window under / near the tray icon
    let _ = win.move_window(Position::TrayBottomCenter);
    let _ = win.show();
    let _ = win.set_focus();

    // macOS: float above all spaces, enable shadow
    #[cfg(target_os = "macos")]
    macos_panel::apply(&win);
}

// ── macOS NSWindow float + shadow ────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod macos_panel {
    //! Minimal AppKit calls to make the window behave like a menu-bar popover:
    //!  - NSWindow level 25 (kCGStatusWindowLevel) — floats above Dock + Spaces
    //!  - Shadow enabled
    //!  - Collection behaviour: CanJoinAllSpaces + FullScreenAuxiliary
    //!
    //! We use the `objc2` crate for safe-ish msg_send! without raw FFI.

    use objc2::{
        msg_send,
        runtime::AnyObject,
    };

    // NSWindowCollectionBehavior bitmask values
    const CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
    const FULL_SCREEN_AUXILIARY: u64 = 1 << 8;

    pub fn apply(win: &tauri::WebviewWindow) {
        // SAFETY: ns_window() returns a valid *mut c_void pointing to an
        // NSWindow on macOS. We only call well-known AppKit methods.
        unsafe {
            let ns_win = win.ns_window() as *mut AnyObject;
            if ns_win.is_null() { return; }

            // Float above Spaces (level 25 = kCGStatusWindowLevel)
            let _: () = msg_send![ns_win, setLevel: 25_i64];

            // Enable drop shadow
            let _: () = msg_send![ns_win, setHasShadow: true];

            // Join all Spaces so the popover shows over fullscreen apps
            let behaviour = CAN_JOIN_ALL_SPACES | FULL_SCREEN_AUXILIARY;
            let _: () = msg_send![ns_win, setCollectionBehavior: behaviour];
        }
    }
}

// ── Windows glyph icon ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod win_icon {
    //! Renders the current planet glyph into a 32×32 RGBA icon using tiny-skia.
    //!
    //! We define each of the 7 Chaldean planet glyphs as tiny-skia stroke paths,
    //! drawn in a 20×20 box then centred on the 32×32 canvas. No font required.

    use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};
    use tauri::image::Image;

    /// Extract the leading Unicode character from status text like "♃ Jupiter · 38m".
    fn first_char(text: &str) -> char {
        text.chars().next().unwrap_or('\u{2643}')
    }

    /// Build a stroke path for a given planet glyph.
    /// The path fits a 20×20 box (origin 0,0).
    fn build_path(ch: char) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        match ch {
            // ☉ Sun: circle + central dot
            '\u{2609}' => {
                pb.push_circle(10.0, 10.0, 9.0);
                pb.push_circle(10.0, 10.0, 2.0);
            }
            // ☽ Moon: crescent (outer circle, then inner offset circle)
            '\u{263D}' => {
                pb.push_circle(10.0, 10.0, 9.0);
                pb.push_circle(13.5, 10.0, 7.0);
            }
            // ♂ Mars: circle + arrow pointing top-right
            '\u{2642}' => {
                pb.push_circle(7.5, 12.5, 7.0);
                pb.move_to(12.5, 7.5);  pb.line_to(19.5, 0.5);
                pb.move_to(14.0, 0.5);  pb.line_to(19.5, 0.5);  pb.line_to(19.5, 6.0);
            }
            // ☿ Mercury: circle + horns + cross + stem
            '\u{263F}' => {
                pb.push_circle(10.0, 10.5, 5.0);
                pb.move_to(5.5, 8.5);  pb.quad_to(10.0, 3.5, 14.5, 8.5);
                pb.move_to(10.0, 15.5); pb.line_to(10.0, 20.0);
                pb.move_to(6.5, 18.0);  pb.line_to(13.5, 18.0);
            }
            // ♃ Jupiter: stylised numeral shape
            '\u{2643}' => {
                pb.move_to(14.0, 1.5);  pb.line_to(14.0, 20.0);
                pb.move_to(5.0, 12.0);  pb.line_to(18.0, 12.0);
                pb.move_to(13.0, 1.5);
                pb.quad_to(3.0, 1.5, 3.0, 8.5);
                pb.quad_to(3.0, 13.0, 14.0, 13.0);
            }
            // ♀ Venus: circle + cross below
            '\u{2640}' => {
                pb.push_circle(10.0, 8.0, 7.0);
                pb.move_to(10.0, 15.0); pb.line_to(10.0, 20.0);
                pb.move_to(6.5, 18.0);  pb.line_to(13.5, 18.0);
            }
            // ♄ Saturn: sickle + cross
            '\u{2644}' => {
                pb.move_to(10.0, 0.5);  pb.line_to(10.0, 14.0);
                pb.move_to(6.0,  4.5);  pb.line_to(14.0, 4.5);
                pb.move_to(10.0, 14.0);
                pb.quad_to(18.0, 14.0, 18.0, 18.5);
                pb.quad_to(18.0, 22.0, 13.5, 21.0);
            }
            _ => return None,
        }
        pb.finish()
    }

    /// Render the planet glyph for `status_text` into a 32×32 Tauri `Image`.
    pub fn render(status_text: &str) -> Option<Image> {
        let ch = first_char(status_text);
        let path = build_path(ch)?;

        let mut pixmap = Pixmap::new(32, 32)?;
        pixmap.fill(Color::TRANSPARENT);

        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);
        paint.anti_alias = true;

        let mut stroke = Stroke::default();
        stroke.width = 1.7;

        // Centre the 20×20 glyph in the 32×32 canvas
        let transform = Transform::from_translate(6.0, 6.0);
        pixmap.stroke_path(&path, &paint, &stroke, transform, None);

        Image::new_owned(pixmap.take(), 32, 32).ok()
    }
}

// ── Periodic refresh (called by timer every 30s) ─────────────────────────────

pub fn refresh_tray(app: &tauri::AppHandle) {
    use astrohour_core::display::status_bar_text;

    let text = {
        let state = app.state::<SharedState>();
        let s = state.lock().unwrap();
        status_bar_text(s.location, s.display_mode)
    };

    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(Some(&text));
        let _ = tray.set_tooltip(Some(&text));

        // Windows: re-render glyph into icon each tick
        #[cfg(target_os = "windows")]
        if let Some(icon) = win_icon::render(&text) {
            let _ = tray.set_icon(Some(icon));
        }
    }

    let _ = app.emit("astro:tick", ());
}

fn initial_tray_text(app: &App) -> String {
    use astrohour_core::display::status_bar_text;
    let state = app.state::<SharedState>();
    let s = state.lock().unwrap();
    status_bar_text(s.location, s.display_mode)
}
