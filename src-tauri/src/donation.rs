//! Donation reminders.
//!
//! AstroHour is free on every platform; donations are what keep it going.
//! Six times a year the app puts up a card inviting the user to donate.
//!
//! The decision is a pure function of the clock — `due_donation(now, state,
//! loc)` — so it can be tested without waiting for September.
//!
//! ## When a reminder fires
//!
//! Most dates fire at the start of the **first Jupiter-ruled day hour**. That
//! is defined by sunrise at the user's location, so everyone gets the card in
//! their own morning rather than at one fixed instant worldwide. It also means
//! the clock time drifts year to year: the dates land on different weekdays,
//! and Jupiter is the first hour on Thursday — its own day — but the fifth on
//! Friday.
//!
//! A date can instead pin a fixed wall-clock time in a named time zone, which
//! is how 2 September 2027 is set.
//!
//! ## The schedule is data
//!
//! Times are chosen year by year rather than by one recurring rule, so the
//! schedule is a lookup by year. It lives in `donation-schedule.json` rather
//! than in this file, so it can be changed without rebuilding — see the
//! [`schedule`] submodule.

use astrohour_core::types::Planet;
use astrohour_core::{planetary_hours, Location};
use chrono::{DateTime, Datelike, Local, Months, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

mod schedule;

pub use schedule::Schedule;

/// How a reminder's moment is determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    /// Start of the first Jupiter-ruled day hour. Depends on sunrise at the
    /// user's location, so the clock time varies by where they are.
    FirstJupiterHour,
    /// A fixed wall-clock time on the date, in the given zone.
    At { hours: u32, minutes: u32, zone: Tz },
}

/// A missed reminder is dropped if the next one is closer than this. Asking
/// about a date already overtaken by the next feels like a debt rather than an
/// invitation.
pub const CATCH_UP_WINDOW_DAYS: i64 = 30;

/// How long the app stays quiet after the user donates.
pub const QUIET_AFTER_DONATION_MONTHS: u32 = 12;

/// One scheduled reminder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub date: NaiveDate,
    /// The instant it becomes due.
    pub due: DateTime<Utc>,
}

impl Occurrence {
    /// Stable identifier for this occurrence, e.g. "2027-09-02". Stored so we
    /// can remember we already asked this year.
    pub fn key(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}",
            self.date.year(),
            self.date.month(),
            self.date.day()
        )
    }
}

/// What we remember between runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DonationState {
    /// Occurrence key of the reminder last put on screen.
    pub last_shown: Option<String>,
    /// "Don't ask again" — permanent.
    pub opted_out: bool,
    /// When the user last clicked Donate, from the card or from Settings.
    pub last_donated: Option<DateTime<Utc>>,
}

/// The instant a given date's reminder becomes due, or `None` if it cannot be
/// determined (an ambiguous Eastern wall time, or a date with no Jupiter hour,
/// neither of which occurs in practice).
fn due_at(date: NaiveDate, when: When, loc: Location) -> Option<DateTime<Utc>> {
    match when {
        When::FirstJupiterHour => planetary_hours(date, loc)
            .into_iter()
            .find(|h| h.is_day && h.planet == Planet::Jupiter)
            .map(|h| h.start.with_timezone(&Utc)),
        When::At {
            hours,
            minutes,
            zone,
        } => zone
            .with_ymd_and_hms(date.year(), date.month(), date.day(), hours, minutes, 0)
            .single()
            .map(|t| t.with_timezone(&Utc)),
    }
}

fn occurrences_in(year: i32, loc: Location, schedule: &Schedule) -> Vec<Occurrence> {
    let mut v: Vec<Occurrence> = schedule
        .for_year(year)
        .iter()
        .filter_map(|((m, d), when)| {
            let date = NaiveDate::from_ymd_opt(year, *m, *d)?;
            Some(Occurrence {
                date,
                due: due_at(date, *when, loc)?,
            })
        })
        .collect();
    v.sort_by_key(|o| o.date);
    v
}

/// Every reminder in the neighbourhood of `today`. Three years is enough to
/// always contain a predecessor and a successor.
fn occurrences_near(today: NaiveDate, loc: Location, schedule: &Schedule) -> Vec<Occurrence> {
    let mut v = Vec::new();
    for y in [today.year() - 1, today.year(), today.year() + 1] {
        v.extend(occurrences_in(y, loc, schedule));
    }
    v.sort_by_key(|o| o.date);
    v
}

fn quiet_after_donation(now: DateTime<Utc>, last: Option<DateTime<Utc>>) -> bool {
    match last {
        Some(t) => match t.checked_add_months(Months::new(QUIET_AFTER_DONATION_MONTHS)) {
            Some(until) => now < until,
            None => false,
        },
        None => false,
    }
}

/// Should the donation card be shown right now, and for which occurrence?
///
/// Two ways a reminder comes due:
///
/// 1. **Today.** It is a reminder date and its moment has passed. Because the
///    test is `now >= due` rather than `now == due`, someone who opens the app
///    hours later still sees it.
/// 2. **Catch-up.** The most recent date went by without the app being open.
///    Shown on the next launch, unless the next date is already close enough
///    that asking now would be noise.
///
/// Either way an occurrence is offered at most once — `last_shown` records the
/// key. Nothing is shown twice, and nothing for a date more than one cycle old.
pub fn due_donation(
    now: DateTime<Utc>,
    state: &DonationState,
    loc: Location,
    schedule: &Schedule,
) -> Option<Occurrence> {
    if state.opted_out {
        return None;
    }
    if quiet_after_donation(now, state.last_donated) {
        return None;
    }

    let today = now.with_timezone(&Local).date_naive();
    let occs = occurrences_near(today, loc, schedule);

    // 1. Today's reminder, once its moment has arrived.
    if let Some(o) = occs.iter().find(|o| o.date == today) {
        if now >= o.due && state.last_shown.as_deref() != Some(o.key().as_str()) {
            return Some(o.clone());
        }
    }

    // 2. Catch up on the most recent missed one, if the next is far enough off.
    let prev = occs.iter().filter(|o| o.date < today).next_back();
    let next = occs.iter().find(|o| o.date >= today);

    if let (Some(prev), Some(next)) = (prev, next) {
        let until_next = (next.date - today).num_days();
        if until_next >= CATCH_UP_WINDOW_DAYS && state.last_shown.as_deref() != Some(prev.key().as_str())
        {
            return Some(prev.clone());
        }
    }

    None
}

// ── Persistence and commands ─────────────────────────────────────────────────

use tauri::Emitter;
use tauri_plugin_store::StoreExt;

/// Event the frontend listens for. Payload is the occurrence key, or an empty
/// string for a preview.
pub const EVENT_DONATION: &str = "astro:donation";

const KEY_LAST_SHOWN: &str = "donation_last_shown";
const KEY_OPTED_OUT: &str = "donation_opted_out";
const KEY_LAST_DONATED: &str = "donation_last_donated";

pub fn load(app: &tauri::AppHandle) -> DonationState {
    let Ok(store) = app.store("prefs.json") else {
        return DonationState::default();
    };
    DonationState {
        last_shown: store
            .get(KEY_LAST_SHOWN)
            .and_then(|v| v.as_str().map(String::from)),
        opted_out: store
            .get(KEY_OPTED_OUT)
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        last_donated: store
            .get(KEY_LAST_DONATED)
            .and_then(|v| v.as_str().map(String::from))
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc)),
    }
}

fn persist(app: &tauri::AppHandle, state: &DonationState) -> Result<(), String> {
    let store = app.store("prefs.json").map_err(|e| e.to_string())?;
    store.set(
        KEY_LAST_SHOWN,
        match &state.last_shown {
            Some(k) => serde_json::Value::String(k.clone()),
            None => serde_json::Value::Null,
        },
    );
    store.set(KEY_OPTED_OUT, serde_json::Value::Bool(state.opted_out));
    store.set(
        KEY_LAST_DONATED,
        match state.last_donated {
            Some(t) => serde_json::Value::String(t.to_rfc3339()),
            None => serde_json::Value::Null,
        },
    );
    store.save().map_err(|e| e.to_string())
}

/// The user's configured location, or `None` if they have not set one.
///
/// Jupiter hours are derived from sunrise, so without a location there is
/// nothing to compute and reminders are skipped rather than guessed at.
fn location_of(app: &tauri::AppHandle) -> Option<Location> {
    use tauri::Manager;
    let state = app.state::<crate::commands::SharedState>();
    let s = state.lock().unwrap();
    if s.location_set {
        Some(s.location)
    } else {
        None
    }
}

/// Check the clock and, if a card is due, put it up.
///
/// The occurrence is recorded as shown *before* the event is emitted. If the
/// app dies with the card on screen we would rather lose one reminder than
/// re-nag on every restart.
pub fn check_due(app: &tauri::AppHandle) {
    let Some(loc) = location_of(app) else {
        return;
    };
    let state = load(app);
    let schedule = Schedule::bundled();
    if let Some(o) = due_donation(Utc::now(), &state, loc, schedule) {
        let mut next = state.clone();
        next.last_shown = Some(o.key());
        let _ = persist(app, &next);
        let _ = app.emit(EVENT_DONATION, o.key());
    }
}

/// "Not now" needs no bookkeeping — the once-per-occurrence rule already
/// silences it until the next date. "Don't ask again" is permanent.
#[tauri::command]
pub fn dismiss_donation(app: tauri::AppHandle, forever: bool) -> Result<(), String> {
    let mut state = load(&app);
    if forever {
        state.opted_out = true;
    }
    persist(&app, &state)
}

/// The user clicked Donate — from the card or from Settings. Either way the
/// quiet period starts now.
#[tauri::command]
pub fn record_donation_click(app: tauri::AppHandle) -> Result<(), String> {
    let mut state = load(&app);
    state.last_donated = Some(Utc::now());
    persist(&app, &state)
}

/// Put the card up now, ignoring the schedule.
///
/// Development only: `debug_assertions` is off in release builds, so the
/// command is not registered there and cannot be reached from a download.
/// In a debug build, call it from the devtools console with
/// `window.__TAURI__.core.invoke('preview_donation')`.
#[cfg(debug_assertions)]
#[tauri::command]
pub fn preview_donation(app: tauri::AppHandle) -> Result<(), String> {
    app.emit(EVENT_DONATION, String::new())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use chrono_tz::America::New_York;

    /// The owner's location. Bedford, Massachusetts — chosen so the due
    /// instants land in the middle of the UTC day, which keeps these tests
    /// independent of the timezone the test runner happens to be in.
    const LOC: Location = Location {
        lat: 42.4604,
        lon: -71.3490,
    };

    fn schedule() -> &'static Schedule {
        Schedule::bundled()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    /// The moment a Jupiter-hour reminder fires on this date.
    fn jupiter_start(d: NaiveDate) -> DateTime<Utc> {
        planetary_hours(d, LOC)
            .into_iter()
            .find(|h| h.is_day && h.planet == Planet::Jupiter)
            .map(|h| h.start.with_timezone(&Utc))
            .expect("every day has a Jupiter day hour")
    }

    fn eastern(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> DateTime<Utc> {
        New_York
            .with_ymd_and_hms(y, m, d, hh, mm, 0)
            .single()
            .unwrap()
            .with_timezone(&Utc)
    }

    fn none() -> DonationState {
        DonationState::default()
    }

    fn key_at(now: DateTime<Utc>) -> Option<String> {
        due_donation(now, &none(), LOC, schedule()).map(|o| o.key())
    }

    // ── The schedule ─────────────────────────────────────────────────────────

    #[test]
    fn the_bundled_schedule_parses() {
        // `bundled` panics if it does not, so simply reaching here is the test.
        let s = schedule();
        assert!(!s.for_year(2027).is_empty());
        assert!(!s.for_year(2028).is_empty());
    }

    #[test]
    fn the_2027_schedule_is_the_six_agreed_dates() {
        let dates: Vec<(u32, u32)> = schedule()
            .for_year(2027)
            .iter()
            .map(|((m, d), _)| (*m, *d))
            .collect();
        assert_eq!(dates, vec![(2, 13), (3, 19), (5, 6), (7, 31), (9, 2), (10, 11)]);
    }

    #[test]
    fn november_25_is_no_longer_scheduled() {
        assert!(!schedule().for_year(2027).iter().any(|((m, d), _)| *m == 11 && *d == 25));
    }

    #[test]
    fn may_6_runs_on_the_jupiter_hour_like_the_rest() {
        let when = schedule()
            .for_year(2027)
            .iter()
            .find(|((m, d), _)| *m == 5 && *d == 6)
            .map(|(_, w)| *w);
        assert_eq!(when, Some(When::FirstJupiterHour));
    }

    #[test]
    fn september_2_pins_8am_eastern() {
        let when = schedule()
            .for_year(2027)
            .iter()
            .find(|((m, d), _)| *m == 9 && *d == 2)
            .map(|(_, w)| *w);
        assert_eq!(
            when,
            Some(When::At { hours: 8, minutes: 0, zone: New_York })
        );
    }

    #[test]
    fn unspecified_years_fall_back_to_the_default() {
        let s = schedule();
        assert_eq!(s.for_year(2028), s.for_year(1999));
    }

    #[test]
    fn every_2027_date_resolves_to_a_moment() {
        for ((m, d), when) in schedule().for_year(2027) {
            assert!(
                due_at(date(2027, *m, *d), *when, LOC).is_some(),
                "{m}-{d} did not resolve"
            );
        }
    }

    // ── Firing ───────────────────────────────────────────────────────────────

    #[test]
    fn not_due_a_second_before_the_moment() {
        let due = jupiter_start(date(2027, 7, 31));
        assert_eq!(key_at(due - Duration::seconds(1)), None);
    }

    #[test]
    fn due_on_the_moment() {
        let due = jupiter_start(date(2027, 7, 31));
        assert_eq!(key_at(due), Some("2027-07-31".into()));
    }

    #[test]
    fn due_hours_later_the_same_day() {
        let due = jupiter_start(date(2027, 7, 31));
        assert_eq!(key_at(due + Duration::hours(6)), Some("2027-07-31".into()));
    }

    #[test]
    fn may_6_fires_at_its_jupiter_hour_not_6am() {
        // 2027: Jupiter's hour runs 05:34-06:45 Eastern, so 06:00 sits inside
        // it and the card must open the hour rather than wait for 6am.
        let due = jupiter_start(date(2027, 5, 6));
        assert!(due < eastern(2027, 5, 6, 6, 0), "Jupiter hour should start before 6am");
        assert_eq!(key_at(due - Duration::seconds(1)), None);
        assert_eq!(key_at(due), Some("2027-05-06".into()));
    }

    #[test]
    fn september_2_ignores_the_jupiter_hour() {
        let pinned = eastern(2027, 9, 2, 8, 0);
        let jupiter = jupiter_start(date(2027, 9, 2));
        assert_ne!(pinned, jupiter, "the pinned time should not coincide");
        assert_eq!(key_at(pinned - Duration::seconds(1)), None);
        assert_eq!(key_at(pinned), Some("2027-09-02".into()));
    }

    #[test]
    fn not_due_on_an_ordinary_day() {
        // 20 Sep: 2 Sep is past but 11 Oct is only 21 days off.
        assert_eq!(key_at(eastern(2027, 9, 20, 12, 0)), None);
    }

    // ── Repetition ───────────────────────────────────────────────────────────

    #[test]
    fn shown_only_once_per_occurrence() {
        let state = DonationState {
            last_shown: Some("2027-09-02".into()),
            ..Default::default()
        };
        assert_eq!(
            due_donation(eastern(2027, 9, 2, 9, 0), &state, LOC, schedule()),
            None
        );
    }

    #[test]
    fn last_years_record_does_not_suppress_this_year() {
        let state = DonationState {
            last_shown: Some("2027-09-02".into()),
            ..Default::default()
        };
        assert_eq!(
            due_donation(eastern(2028, 9, 2, 12, 0), &state, LOC, schedule())
                .map(|o| o.key()),
            Some("2028-09-02".into())
        );
    }

    #[test]
    fn dont_ask_again_silences_everything() {
        let state = DonationState {
            opted_out: true,
            ..Default::default()
        };
        assert_eq!(due_donation(eastern(2027, 9, 2, 12, 0), &state, LOC, schedule()), None);
        assert_eq!(due_donation(eastern(2030, 2, 13, 12, 0), &state, LOC, schedule()), None);
    }

    // ── The quiet period after donating ──────────────────────────────────────

    #[test]
    fn donating_silences_for_a_year() {
        let state = DonationState {
            last_donated: Some(eastern(2027, 9, 2, 8, 30)),
            ..Default::default()
        };
        assert_eq!(due_donation(eastern(2027, 10, 11, 12, 0), &state, LOC, schedule()), None);
        assert_eq!(due_donation(eastern(2028, 2, 13, 12, 0), &state, LOC, schedule()), None);
    }

    #[test]
    fn next_reminder_is_the_first_date_after_the_anniversary() {
        // Donated 2 Sep 2027 -> quiet until 2 Sep 2028, which is itself a
        // reminder date, so that is the next card.
        let state = DonationState {
            last_donated: Some(eastern(2027, 9, 2, 8, 30)),
            ..Default::default()
        };
        let anniversary = eastern(2028, 9, 2, 8, 30);
        assert_eq!(
            due_donation(anniversary - Duration::minutes(1), &state, LOC, schedule()),
            None
        );
        assert_eq!(
            due_donation(anniversary, &state, LOC, schedule()).map(|o| o.key()),
            Some("2028-09-02".into())
        );
    }

    #[test]
    fn donating_off_schedule_waits_for_the_next_date() {
        // Donated 5 Sep 2027 (not a reminder date) -> quiet until 5 Sep 2028,
        // so the next card is 11 Oct 2028.
        let state = DonationState {
            last_donated: Some(eastern(2027, 9, 5, 12, 0)),
            ..Default::default()
        };
        assert_eq!(due_donation(eastern(2028, 9, 2, 12, 0), &state, LOC, schedule()), None);
        assert_eq!(
            due_donation(eastern(2028, 10, 11, 12, 0), &state, LOC, schedule())
                .map(|o| o.key()),
            Some("2028-10-11".into())
        );
    }

    // ── Catch-up ─────────────────────────────────────────────────────────────

    #[test]
    fn missed_reminder_is_shown_on_the_next_launch() {
        // App closed all day 31 Jul 2027; opened 1 Aug. Next date (2 Sep) is
        // 32 days away, so the missed one is still worth showing.
        assert_eq!(key_at(eastern(2027, 8, 1, 12, 0)), Some("2027-07-31".into()));
    }

    #[test]
    fn missed_reminder_is_dropped_when_the_next_one_is_close() {
        // Opened 10 Aug; 2 Sep is only 23 days off, so skip the stale one.
        assert_eq!(key_at(eastern(2027, 8, 10, 12, 0)), None);
    }

    #[test]
    fn catch_up_holds_right_up_to_the_window() {
        // 3 Aug leaves exactly 30 days to 2 Sep, so it is still shown.
        assert_eq!(key_at(eastern(2027, 8, 3, 12, 0)), Some("2027-07-31".into()));
        // 1 Sep is one day short of the next date, so nothing is due.
        assert_eq!(key_at(eastern(2027, 9, 1, 12, 0)), None);
    }

    #[test]
    fn a_short_gap_date_is_rarely_caught_up() {
        // 13 Feb -> 19 Mar is only 34 days, so the window closes after four.
        assert_eq!(key_at(eastern(2027, 2, 15, 12, 0)), Some("2027-02-13".into()));
        assert_eq!(key_at(eastern(2027, 2, 20, 12, 0)), None);
    }

    #[test]
    fn no_catch_up_on_the_day_itself_before_its_moment() {
        // 2 Sep at 07:00 Eastern: today's 08:00 card is not due yet, and the
        // previous date is too stale to chase.
        assert_eq!(key_at(eastern(2027, 9, 2, 7, 0)), None);
    }

    #[test]
    fn catch_up_is_offered_only_once() {
        let state = DonationState {
            last_shown: Some("2027-07-31".into()),
            ..Default::default()
        };
        assert_eq!(
            due_donation(eastern(2027, 8, 1, 12, 0), &state, LOC, schedule()),
            None
        );
    }
}
