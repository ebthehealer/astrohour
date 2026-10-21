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
//! by `win_icon::render()` using embedded Symbola-font PNGs, tinted at runtime.

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

    let mut builder = TrayIconBuilder::with_id("main")
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

    // Platform-specific icon setup
    #[cfg(target_os = "windows")]
    if let Some(icon) = win_icon::render(&initial_text) {
        builder = builder.icon(icon);
    }

    #[cfg(target_os = "macos")]
    { builder = builder.icon_as_template(true); }

    #[cfg(target_os = "linux")]
    {
        let tmp = std::env::temp_dir().join("astrohour-tray-0");
        let _ = std::fs::create_dir_all(&tmp);
        let icon = linux_icon::render_to_dir(&initial_text, &tmp, "white")
            .unwrap_or_else(|| {
                tauri::image::Image::from_bytes(include_bytes!("../icons/tray-planet.png"))
                    .expect("fallback tray icon missing")
            });
        builder = builder.icon(icon);
    }

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

    // TrayCenter positions above the tray icon using the rect recorded by
    // on_tray_event(). On Linux/libappindicator that rect may be (0,0) so
    // the window ends up at the top-left corner. After positioning, check
    // if we're stuck near (0,0) and fall back to BottomRight.
    let _ = win.move_window(Position::TrayCenter);

    #[cfg(target_os = "linux")]
    {
        // Give GTK a moment to apply the position, then sanity-check it.
        let pos = win.outer_position().unwrap_or_default();
        if pos.x < 50 && pos.y < 50 {
            let _ = win.move_window(Position::BottomRight);
        }
    }

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


// ── Shared planet icon (PNG-based, both Linux and Windows) ─────────────────────
//
// Each planet glyph is a 32×32 white-on-transparent PNG generated from the
// Symbola font. At render time we tint the white pixels to the chosen color.

mod planet_icon {
    // Embed all 7 PNGs at compile time
    const SUN:     &[u8] = include_bytes!("../icons/planets/sun.png");
    const MOON:    &[u8] = include_bytes!("../icons/planets/moon.png");
    const MARS:    &[u8] = include_bytes!("../icons/planets/mars.png");
    const MERCURY: &[u8] = include_bytes!("../icons/planets/mercury.png");
    const JUPITER: &[u8] = include_bytes!("../icons/planets/jupiter.png");
    const VENUS:   &[u8] = include_bytes!("../icons/planets/venus.png");
    const SATURN:  &[u8] = include_bytes!("../icons/planets/saturn.png");

    fn png_for(text: &str) -> &'static [u8] {
        match text.chars().next().unwrap_or('♃') {
            '\u{2609}' => SUN,
            '\u{263D}' => MOON,
            '\u{2642}' => MARS,
            '\u{263F}' => MERCURY,
            '\u{2643}' => JUPITER,
            '\u{2640}' => VENUS,
            '\u{2644}' => SATURN,
            _          => JUPITER, // fallback
        }
    }

    pub fn parse_color(color: &str) -> (u8, u8, u8) {
        match color.trim() {
            "auto" | "white" => (255, 255, 255),
            "gold"           => (200, 168,  75),
            s if s.starts_with('#') && s.len() == 7 => {
                let r = u8::from_str_radix(&s[1..3], 16).unwrap_or(255);
                let g = u8::from_str_radix(&s[3..5], 16).unwrap_or(255);
                let b = u8::from_str_radix(&s[5..7], 16).unwrap_or(255);
                (r, g, b)
            }
            _ => (255, 255, 255),
        }
    }

    /// Decode PNG → RGBA, tint white pixels to `color`, return raw bytes.
    pub fn render_rgba(status_text: &str, color: &str) -> Option<(Vec<u8>, u32, u32)> {
        let png = png_for(status_text);
        // Decode PNG manually using the `png` crate (already a transitive dep via tauri)
        let cursor = std::io::Cursor::new(png);
        let decoder = png::Decoder::new(cursor);
        let mut reader = decoder.read_info().ok()?;
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).ok()?;
        let w = info.width;
        let h = info.height;
        let bytes = &buf[..info.buffer_size()];

        let (tr, tg, tb) = parse_color(color);

        // Convert to RGBA and tint: replace RGB of non-transparent pixels
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        match info.color_type {
            png::ColorType::Rgba => {
                for chunk in bytes.chunks_exact(4) {
                    let a = chunk[3];
                    if a > 0 {
                        rgba.extend_from_slice(&[tr, tg, tb, a]);
                    } else {
                        rgba.extend_from_slice(&[0, 0, 0, 0]);
                    }
                }
            }
            png::ColorType::Rgb => {
                for chunk in bytes.chunks_exact(3) {
                    // treat brightness as alpha
                    let brightness = chunk[0].max(chunk[1]).max(chunk[2]);
                    rgba.extend_from_slice(&[tr, tg, tb, brightness]);
                }
            }
            png::ColorType::Grayscale => {
                for &g in bytes {
                    rgba.extend_from_slice(&[tr, tg, tb, g]);
                }
            }
            png::ColorType::GrayscaleAlpha => {
                for chunk in bytes.chunks_exact(2) {
                    rgba.extend_from_slice(&[tr, tg, tb, chunk[1]]);
                }
            }
            _ => return None,
        }

        Some((rgba, w, h))
    }

}

// ── Windows tray icon ───────────────────────────────────────────────────────────
#[cfg(target_os = "windows")]
mod win_icon {
    use tauri::image::Image;
    pub fn render(status_text: &str) -> Option<Image<'static>> {
        super::planet_icon::render(status_text, "white")
    }
}

// ── Linux tray icon ─────────────────────────────────────────────────────────────
#[cfg(target_os = "linux")]
mod linux_icon {
    use tauri::image::Image;

    pub fn render_rgba(status_text: &str, color: &str) -> Option<(Vec<u8>, u32, u32)> {
        super::planet_icon::render_rgba(status_text, color)
    }

    pub fn render_to_dir(status_text: &str, dir: &std::path::Path, color: &str) -> Option<Image<'static>> {
        // Write tinted PNG to dir/icon.png for the initial tray setup
        let (rgba, w, h) = render_rgba(status_text, color)?;
        // Re-encode as PNG
        let file = dir.join("icon.png");
        let f = std::fs::File::create(&file).ok()?;
        let mut enc = png::Encoder::new(f, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().ok()?;
        writer.write_image_data(&rgba).ok()?;
        drop(writer);
        tauri::image::Image::from_path(&file).ok()
    }
}

// ── Tray refresh ───────────────────────────────────────────────────────────────

pub fn refresh_tray(app: &tauri::AppHandle) {
    use astrohour_core::display::status_bar_text;
    use astrohour_core::hours::current_hour;
    use chrono::Local;

    let (text, color, tooltip) = {
        let state = app.state::<SharedState>();
        let s = state.lock().unwrap();
        let bar   = status_bar_text(s.location, s.display_mode);
        let color = s.tray_icon_color.clone();

        // Build a rich tooltip: "♃ Jupiter Hour · 38m remaining (7:12 AM – 8:06 AM)"
        let hour       = current_hour(s.location);
        let now        = Local::now();
        let mins_left  = hour.minutes_remaining_from(now);
        let glyph      = hour.planet.symbol();
        let name       = hour.planet.name();
        let fmt        = if s.clock_format == "24" { "%H:%M" } else { "%-I:%M %p" };
        let start_str  = hour.start.format(fmt);
        let end_str    = hour.end.format(fmt);
        let mins_int   = mins_left.round() as i64;
        let tip = format!(
            "{glyph} {name} Hour · {mins_int}m remaining ({start_str} – {end_str})"
        );

        (bar, color, tip)
    };

    eprintln!("[tray] refresh: text={:?} color={:?}", text, color);

    #[cfg(target_os = "linux")]
    let icon_bytes: Option<(Vec<u8>, u32, u32)> = linux_icon::render_rgba(&text, &color);

    #[cfg(not(target_os = "linux"))]
    let icon_bytes: Option<(Vec<u8>, u32, u32)> = None;

    let app2    = app.clone();
    let text2   = text.clone();
    let tip2    = tooltip.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = app2.tray_by_id("main") else {
            eprintln!("[tray] ERROR: tray_by_id('main') returned None on main thread");
            return;
        };

        let _ = tray.set_title(Some(&text2));
        let _ = tray.set_tooltip(Some(&tip2));

        #[cfg(target_os = "windows")]
        if let Some(icon) = win_icon::render(&text2) {
            let _ = tray.set_icon(Some(icon));
        }

        #[cfg(target_os = "linux")]
        if let Some((rgba, w, h)) = icon_bytes {
            use tauri::image::Image;
            let img = Image::new_owned(rgba, w, h);
            match tray.set_icon(Some(img)) {
                Ok(_)  => eprintln!("[tray] set_icon ok"),
                Err(e) => eprintln!("[tray] set_icon ERROR: {:?}", e),
            }
        } else {
            eprintln!("[tray] no icon_bytes to set");
        }
    });

    let _ = app.emit("astro:tick", ());
}

fn initial_tray_text(app: &App) -> String {
    use astrohour_core::display::status_bar_text;
    let state = app.state::<SharedState>();
    let s = state.lock().unwrap();
    status_bar_text(s.location, s.display_mode)
}
