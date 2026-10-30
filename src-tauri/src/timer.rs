//! Background refresh timer.
//!
//! Two threads:
//!
//! * a 10-second watcher that detects planetary-hour boundaries and fires
//!   "N minutes left" reminders, and
//! * a 30-second heartbeat that keeps the tray tooltip and title current.
//!
//! Both derive the current hour from the frontend-supplied UTC offset so the
//! notification never disagrees with what the window is showing.

use tauri::{AppHandle, Manager};
use std::collections::HashSet;
use std::time::Duration;
use astrohour_core::types::PlanetaryHour;
use crate::commands::{current_hour_and_date, SharedState};

pub fn start_refresh_timer(app: AppHandle) {
    let app_heartbeat = app.clone();

    // ── Hour-boundary + reminder watcher (10 s resolution) ────────────────────
    std::thread::spawn(move || {
        let mut last_hour_start: Option<i64> = None;
        // Reminders already announced, keyed by (hour start unix, lead minutes).
        // Keeps a long-running session from repeating itself.
        let mut fired: HashSet<(i64, u32)> = HashSet::new();

        loop {
            std::thread::sleep(Duration::from_secs(10));

            let (hour, reminders, notify_on, hour_sound, reminder_sound) = {
                let state = app.state::<SharedState>();
                let s = state.lock().unwrap();
                (
                    current_hour_and_date(s.location, s.utc_offset_mins).0,
                    s.reminder_minutes.clone(),
                    s.notifications_enabled,
                    s.hour_sound.clone(),
                    s.reminder_sound.clone(),
                )
            };

            let hour_start = hour.start.timestamp();
            let first_tick = last_hour_start.is_none();
            let hour_changed = last_hour_start.is_some_and(|prev| prev != hour_start);

            if first_tick || hour_changed {
                // Drop bookkeeping for hours we've moved past.
                fired.retain(|(ts, _)| *ts == hour_start);
            }

            if hour_changed {
                // Refresh tray icon + tooltip immediately at the boundary.
                crate::tray::refresh_tray(&app);
                if notify_on {
                    send_hour_notification(&app, &hour, &hour_sound);
                }
            }

            if notify_on && !reminders.is_empty() {
                let due = due_reminders(
                    hour_start,
                    seconds_left(&hour),
                    &reminders,
                    &mut fired,
                    first_tick,
                );
                for m in due {
                    send_reminder_notification(&app, &hour, m, &reminder_sound);
                }
            }

            last_hour_start = Some(hour_start);
        }
    });

    // ── 30-second heartbeat — keeps tooltip/title current ─────────────────────
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        crate::tray::refresh_tray(&app_heartbeat);
        // Donation reminders ride the same heartbeat. Cheap: it reads three
        // keys from the prefs store and compares dates.
        crate::donation::check_due(&app_heartbeat);
    });
}

/// Whole seconds until this hour ends. Negative once it has.
fn seconds_left(hour: &PlanetaryHour) -> i64 {
    hour.end.signed_duration_since(chrono::Utc::now()).num_seconds()
}

/// Which reminder lead times are due on this tick, marking them as announced.
///
/// `fired` is keyed by `(hour start, lead minutes)` so a reminder fires at most
/// once per hour. On the first tick after launch any lead time whose moment has
/// already passed is recorded without firing — otherwise starting the app with
/// four minutes left in an hour would dump every pending reminder at once.
fn due_reminders(
    hour_start: i64,
    remaining: i64,
    reminders: &[u32],
    fired: &mut HashSet<(i64, u32)>,
    first_tick: bool,
) -> Vec<u32> {
    if first_tick {
        for &m in reminders {
            if remaining < i64::from(m) * 60 {
                fired.insert((hour_start, m));
            }
        }
    }

    reminders
        .iter()
        .copied()
        .filter(|&m| {
            let key = (hour_start, m);
            if fired.contains(&key) {
                return false;
            }
            if remaining > 0 && remaining <= i64::from(m) * 60 {
                fired.insert(key);
                return true;
            }
            false
        })
        .collect()
}

/// Clock format string for the user's preference.
fn clock_fmt(app: &AppHandle) -> String {
    let state = app.state::<SharedState>();
    let s = state.lock().unwrap();
    if s.clock_format == "24" { "%H:%M".into() } else { "%-I:%M %p".into() }
}

/// Fire a desktop notification announcing a new planetary hour.
fn send_hour_notification(app: &AppHandle, hour: &PlanetaryHour, sound: &str) {
    let fmt = clock_fmt(app);

    let glyph     = hour.planet.symbol();   // ♃  ♄  ☉  etc.
    let name      = hour.planet.name();     // Jupiter
    let start_str = hour.start.format(&fmt).to_string();
    let end_str   = hour.end.format(&fmt).to_string();
    let day_night = if hour.is_day { "Day" } else { "Night" };
    let seq       = hour.sequence;          // 1-12

    let title = format!("{glyph} {name} Hour");
    let body  = format!("{day_night} hour {seq} · {start_str} – {end_str}");

    notify(app, &title, &body, sound);
}

/// Fire a desktop notification warning that the current hour is nearly over.
fn send_reminder_notification(app: &AppHandle, hour: &PlanetaryHour, minutes: u32, sound: &str) {
    let fmt = clock_fmt(app);

    let glyph     = hour.planet.symbol();
    let name      = hour.planet.name();
    let end_str   = hour.end.format(&fmt).to_string();
    let day_night = if hour.is_day { "Day" } else { "Night" };
    let seq       = hour.sequence;

    let unit  = if minutes == 1 { "minute" } else { "minutes" };
    let title = format!("{glyph} {name} Hour — {minutes} {unit} left");
    let body  = format!("Ends at {end_str} · {day_night} hour {seq} of 12");

    notify(app, &title, &body, sound);
}

/// Show a notification with the app icon, then play the chosen sound.
///
/// The `"system"` sound is delegated to the notification daemon through the
/// freedesktop `sound-name` hint; every other sound is played by us so the
/// result is identical regardless of which daemon is running.
fn notify(app: &AppHandle, title: &str, body: &str, sound: &str) {
    use tauri_plugin_notification::NotificationExt;

    let mut builder = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        // Matches the hicolor icon installed by install-dev-desktop.sh and the
        // Linux packages, so the notification shows the Saturn mark.
        .icon("com.astrohour.app");

    if sound == "system" {
        builder = builder.sound("message");
    }

    if let Err(e) = builder.show() {
        eprintln!("[notify] failed to show notification: {e:?}");
    }

    if sound != "system" {
        crate::sound::play(sound);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 1_700_000_000;

    /// Walk a whole hour in 10-second ticks and report when each reminder fired.
    fn run_hour(reminders: &[u32], first_tick_remaining: i64) -> Vec<(i64, u32)> {
        let mut fired = HashSet::new();
        let mut events = Vec::new();

        let mut remaining = first_tick_remaining;
        let mut first = true;
        while remaining >= 0 {
            let due = due_reminders(HOUR, remaining, reminders, &mut fired, first);
            for m in due {
                events.push((remaining, m));
            }
            first = false;
            remaining -= 10;
        }
        events
    }

    #[test]
    fn reminder_fires_once_near_its_lead_time() {
        let events = run_hour(&[5, 10, 30], 3600);
        assert_eq!(events.len(), 3, "one shot per lead time: {events:?}");

        for (remaining, minutes) in &events {
            // 10 s tick granularity, so we land within a tick of the threshold.
            let expected = i64::from(*minutes) * 60;
            assert!(
                *remaining <= expected && *remaining > expected - 10,
                "{minutes}m fired with {remaining}s left, expected ~{expected}"
            );
        }
    }

    #[test]
    fn reminders_fire_in_descending_lead_order() {
        let events = run_hour(&[5, 10, 15, 20, 30], 3600);
        let minutes: Vec<u32> = events.iter().map(|(_, m)| *m).collect();
        assert_eq!(minutes, vec![30, 20, 15, 10, 5]);
    }

    #[test]
    fn no_reminders_when_none_selected() {
        assert!(run_hour(&[], 3600).is_empty());
    }

    #[test]
    fn launching_late_does_not_replay_missed_reminders() {
        // App started with 4 minutes left: the 5/10/30-minute moments are all
        // in the past, so nothing should pop up at launch.
        let events = run_hour(&[5, 10, 30], 240);
        assert!(events.is_empty(), "unexpected launch spam: {events:?}");
    }

    #[test]
    fn launching_late_still_fires_upcoming_reminders() {
        // 12 minutes left: 30 and 15 are gone, 10 and 5 are still ahead.
        let events = run_hour(&[5, 10, 15, 30], 720);
        let minutes: Vec<u32> = events.iter().map(|(_, m)| *m).collect();
        assert_eq!(minutes, vec![10, 5]);
    }

    #[test]
    fn firing_exactly_on_the_boundary_counts() {
        let mut fired = HashSet::new();
        let due = due_reminders(HOUR, 300, &[5], &mut fired, false);
        assert_eq!(due, vec![5]);

        // …and not again a tick later.
        let due = due_reminders(HOUR, 290, &[5], &mut fired, false);
        assert!(due.is_empty());
    }

    #[test]
    fn an_ended_hour_never_fires() {
        let mut fired = HashSet::new();
        assert!(due_reminders(HOUR, 0, &[5], &mut fired, false).is_empty());
        assert!(due_reminders(HOUR, -30, &[5], &mut fired, false).is_empty());
    }

    #[test]
    fn bookkeeping_is_scoped_per_hour() {
        let mut fired = HashSet::new();
        assert_eq!(due_reminders(HOUR, 300, &[5], &mut fired, false), vec![5]);

        // Next hour, same lead time — should fire again.
        fired.retain(|(ts, _)| *ts == HOUR + 3600);
        assert_eq!(due_reminders(HOUR + 3600, 300, &[5], &mut fired, false), vec![5]);
    }

    #[test]
    fn enabling_a_new_lead_time_mid_hour_fires_it() {
        // User ticks "10 min" while 4 minutes remain: it's already passed, but
        // unlike a fresh launch we do fire it — they just asked for it.
        let mut fired = HashSet::new();
        let due = due_reminders(HOUR, 240, &[10], &mut fired, false);
        assert_eq!(due, vec![10]);
    }
}
