//! Donation reminders.
//!
//! AstroHour is free on every platform; donations are what keep it going.
//! Seven times a year the app puts up a card inviting the user to donate.
//!
//! The whole decision is a pure function of the clock — `due_donation(now,
//! state)` — so it can be tested without waiting for September.
//!
//! ## Timezone
//!
//! The dates and times are specified in **US Eastern**, not the user's local
//! time. The dates were chosen deliberately, so the moment is anchored to one
//! reference zone rather than drifting per user. The consequence is worth
//! knowing: someone in Tokyo is shown the card at around 10pm local, and
//! someone in Sydney around midnight. If that becomes a problem, the fix is to
//! compare against the user's local clock instead of `New_York` below — the
//! rest of the module does not care which zone it is given.

use chrono::{DateTime, Datelike, Months, NaiveDate, Timelike, Utc};
use chrono_tz::America::New_York;

/// The seven days a year the card appears, and the Eastern time it becomes due.
///
/// Calendar order, which is not the order they were chosen in. Gaps between
/// consecutive dates are 34, 48, 86, 33, 39, 45 and 80 days.
pub const DONATION_DATES: &[((u32, u32), (u32, u32))] = &[
    ((2, 13), (9, 0)),
    ((3, 19), (10, 0)),
    ((5, 6), (6, 0)),
    ((7, 31), (10, 0)),
    ((9, 2), (8, 0)),
    ((10, 11), (10, 0)),
    ((11, 25), (10, 0)),
];

/// A missed reminder is dropped if the next one is closer than this. Asking
/// about a date that has already been overtaken by the next feels like a debt
/// rather than an invitation.
pub const CATCH_UP_WINDOW_DAYS: i64 = 30;

/// How long the app stays quiet after the user donates.
pub const QUIET_AFTER_DONATION_MONTHS: u32 = 12;

/// One scheduled reminder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub date: NaiveDate,
    /// Eastern time of day it becomes due.
    pub time: (u32, u32),
}

impl Occurrence {
    /// Stable identifier for this occurrence, e.g. "2026-09-02". Stored so we
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

fn occurrences_in(year: i32) -> Vec<Occurrence> {
    let mut v: Vec<Occurrence> = DONATION_DATES
        .iter()
        .filter_map(|((m, d), t)| {
            NaiveDate::from_ymd_opt(year, *m, *d).map(|date| Occurrence { date, time: *t })
        })
        .collect();
    v.sort_by_key(|o| o.date);
    v
}

/// Every reminder in the neighbourhood of `today`. Three years is enough to
/// always contain a predecessor and a successor.
fn occurrences_near(today: NaiveDate) -> Vec<Occurrence> {
    let mut v = Vec::new();
    for y in [today.year() - 1, today.year(), today.year() + 1] {
        v.extend(occurrences_in(y));
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
/// 1. **Today.** It is a donation date and its time has passed. Because the
///    test is `now >= time` rather than `now == time`, someone who opens the
///    app at 6pm on a donation date still sees it.
/// 2. **Catch-up.** The most recent date went by without the app being open.
///    Shown on the next launch, unless the next date is already close enough
///    that asking now would be noise.
///
/// Either way, an occurrence is offered at most once — `last_shown` records
/// the key. Nothing is ever shown twice, and nothing is shown for a date more
/// than one cycle old.
pub fn due_donation(now: DateTime<Utc>, state: &DonationState) -> Option<Occurrence> {
    if state.opted_out {
        return None;
    }
    if quiet_after_donation(now, state.last_donated) {
        return None;
    }

    let local = now.with_timezone(&New_York);
    let today = local.date_naive();
    let occs = occurrences_near(today);

    // 1. Today's reminder, once its time has arrived.
    if let Some(o) = occs.iter().find(|o| o.date == today) {
        let now_hm = (local.hour(), local.minute());
        if now_hm >= o.time && state.last_shown.as_deref() != Some(o.key().as_str()) {
            return Some(o.clone());
        }
    }

    // 2. Catch up on the most recent missed one, if the next is far enough off.
    let prev = occs.iter().filter(|o| o.date < today).next_back();
    let next = occs.iter().find(|o| o.date >= today);

    if let (Some(prev), Some(next)) = (prev, next) {
        let until_next = (next.date - today).num_days();
        if until_next >= CATCH_UP_WINDOW_DAYS && state.last_shown.as_deref() != Some(prev.key().as_str()) {
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

/// Check the clock and, if a card is due, put it up.
///
/// The occurrence is recorded as shown *before* the event is emitted. If the
/// app dies with the card on screen we would rather lose one reminder than
/// re-nag on every restart.
pub fn check_due(app: &tauri::AppHandle) {
    let state = load(app);
    if let Some(o) = due_donation(Utc::now(), &state) {
        let mut next = state.clone();
        next.last_shown = Some(o.key());
        let _ = persist(app, &next);
        let _ = app.emit(EVENT_DONATION, o.key());
    }
}

/// The frontend has the card on screen.
#[tauri::command]
pub fn mark_donation_shown(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let mut state = load(&app);
    state.last_shown = Some(key);
    persist(&app, &state)
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

/// Put the card up now, ignoring the schedule. For previewing.
#[tauri::command]
pub fn preview_donation(app: tauri::AppHandle) -> Result<(), String> {
    app.emit(EVENT_DONATION, String::new())
        .map_err(|e| e.to_string())
}

/// When the next card is due, for display in Settings.
#[tauri::command]
pub fn next_donation_date(app: tauri::AppHandle) -> Option<String> {
    let state = load(&app);
    if state.opted_out {
        return None;
    }
    let today = Utc::now().with_timezone(&New_York).date_naive();
    occurrences_near(today)
        .into_iter()
        .find(|o| o.date >= today)
        .map(|o| o.key())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// An instant expressed as Eastern wall-clock — how the schedule is written.
    fn eastern(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> DateTime<Utc> {
        New_York
            .with_ymd_and_hms(y, m, d, hh, mm, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn none() -> DonationState {
        DonationState::default()
    }

    fn key_of(now: DateTime<Utc>) -> Option<String> {
        due_donation(now, &none()).map(|o| o.key())
    }

    #[test]
    fn the_schedule_is_the_seven_agreed_dates() {
        let got: Vec<((u32, u32), (u32, u32))> = DONATION_DATES.to_vec();
        assert_eq!(
            got,
            vec![
                ((2, 13), (9, 0)),
                ((3, 19), (10, 0)),
                ((5, 6), (6, 0)),
                ((7, 31), (10, 0)),
                ((9, 2), (8, 0)),
                ((10, 11), (10, 0)),
                ((11, 25), (10, 0)),
            ]
        );
    }

    #[test]
    fn not_due_before_the_configured_time() {
        assert_eq!(key_of(eastern(2026, 9, 2, 7, 59)), None);
    }

    #[test]
    fn due_at_the_configured_time() {
        assert_eq!(key_of(eastern(2026, 9, 2, 8, 0)), Some("2026-09-02".into()));
    }

    #[test]
    fn due_later_the_same_day() {
        assert_eq!(key_of(eastern(2026, 9, 2, 23, 30)), Some("2026-09-02".into()));
    }

    #[test]
    fn the_day_after_a_missed_date_is_the_catch_up_case() {
        // App closed all day 2 Sep; opened 3 Sep. 11 Oct is 38 days off, so
        // the missed card is shown now.
        assert_eq!(key_of(eastern(2026, 9, 3, 12, 0)), Some("2026-09-02".into()));
    }

    #[test]
    fn nothing_is_due_once_the_window_has_closed() {
        // 20 Sep: 2 Sep is past, but 11 Oct is only 21 days off.
        assert_eq!(key_of(eastern(2026, 9, 20, 12, 0)), None);
        // 5 Jul: 6 May is long gone and 31 Jul is 26 days off.
        assert_eq!(key_of(eastern(2026, 7, 5, 12, 0)), None);
    }

    #[test]
    fn every_date_has_a_plausible_time() {
        for ((m, d), (hh, mm)) in DONATION_DATES {
            assert!(*hh < 24 && *mm < 60, "{m}-{d} has an invalid time");
        }
    }

    #[test]
    fn shown_only_once_per_occurrence() {
        let state = DonationState {
            last_shown: Some("2026-09-02".into()),
            ..Default::default()
        };
        assert_eq!(due_donation(eastern(2026, 9, 2, 11, 0), &state), None);
    }

    #[test]
    fn last_years_record_does_not_suppress_this_year() {
        let state = DonationState {
            last_shown: Some("2026-09-02".into()),
            ..Default::default()
        };
        assert_eq!(
            due_donation(eastern(2027, 9, 2, 8, 0), &state).map(|o| o.key()),
            Some("2027-09-02".into())
        );
    }

    #[test]
    fn dont_ask_again_silences_everything() {
        let state = DonationState {
            opted_out: true,
            ..Default::default()
        };
        assert_eq!(due_donation(eastern(2026, 9, 2, 12, 0), &state), None);
        assert_eq!(due_donation(eastern(2030, 11, 25, 12, 0), &state), None);
    }

    #[test]
    fn donating_silences_for_a_year() {
        let state = DonationState {
            last_donated: Some(eastern(2026, 9, 2, 8, 30)),
            ..Default::default()
        };
        // Oct 11 and Nov 25 of the same year are skipped.
        assert_eq!(due_donation(eastern(2026, 10, 11, 10, 0), &state), None);
        assert_eq!(due_donation(eastern(2026, 11, 25, 10, 0), &state), None);
        assert_eq!(due_donation(eastern(2027, 2, 13, 9, 0), &state), None);
    }

    #[test]
    fn next_reminder_is_the_first_date_after_the_anniversary() {
        // Donated on 2 Sep 2026 -> quiet until 2 Sep 2027, which is itself a
        // donation date, so that is the next card.
        let state = DonationState {
            last_donated: Some(eastern(2026, 9, 2, 8, 30)),
            ..Default::default()
        };
        assert_eq!(due_donation(eastern(2027, 9, 2, 7, 0), &state), None);
        // The quiet period runs to the same time of day the donation was made
        // (08:30), which is after that date's own 08:00 start, so the card
        // waits half an hour rather than appearing with the schedule.
        assert_eq!(due_donation(eastern(2027, 9, 2, 8, 0), &state), None);
        assert_eq!(
            due_donation(eastern(2027, 9, 2, 8, 30), &state).map(|o| o.key()),
            Some("2027-09-02".into())
        );
    }

    #[test]
    fn donating_off_schedule_waits_for_the_next_date() {
        // Donated 5 Sep 2026 (not a donation date) -> quiet until 5 Sep 2027,
        // so the next card is 11 Oct 2027.
        let state = DonationState {
            last_donated: Some(eastern(2026, 9, 5, 12, 0)),
            ..Default::default()
        };
        assert_eq!(due_donation(eastern(2027, 9, 2, 8, 0), &state), None);
        assert_eq!(
            due_donation(eastern(2027, 10, 11, 10, 0), &state).map(|o| o.key()),
            Some("2027-10-11".into())
        );
    }

    #[test]
    fn missed_reminder_is_shown_on_the_next_launch() {
        // App closed all day 31 Jul 2026; opened 1 Aug. Next date (2 Sep) is
        // 32 days away, so the missed one is still worth showing.
        assert_eq!(key_of(eastern(2026, 8, 1, 12, 0)), Some("2026-07-31".into()));
    }

    #[test]
    fn missed_reminder_is_dropped_when_the_next_one_is_close() {
        // Opened 10 Aug; 2 Sep is only 23 days off, so skip the stale one.
        assert_eq!(key_of(eastern(2026, 8, 10, 12, 0)), None);
    }

    #[test]
    fn catch_up_holds_right_up_to_the_window() {
        // 31 Jul + 32 days = 1 Sep, exactly 1 day before the next date.
        assert_eq!(key_of(eastern(2026, 9, 1, 12, 0)), None);
        // 3 Aug leaves 30 days to 2 Sep: exactly at the window, so shown.
        assert_eq!(key_of(eastern(2026, 8, 3, 12, 0)), Some("2026-07-31".into()));
    }

    #[test]
    fn a_short_gap_date_is_rarely_caught_up() {
        // 13 Feb -> 19 Mar is only 34 days, so the catch-up window closes
        // after four days.
        assert_eq!(key_of(eastern(2026, 2, 15, 12, 0)), Some("2026-02-13".into()));
        assert_eq!(key_of(eastern(2026, 2, 20, 12, 0)), None);
    }

    #[test]
    fn no_catch_up_on_the_day_itself_before_its_time() {
        // 2 Sep at 7am: today's card is not due yet, and the previous date is
        // too stale to chase.
        assert_eq!(key_of(eastern(2026, 9, 2, 7, 0)), None);
    }

    #[test]
    fn catch_up_is_offered_only_once() {
        let state = DonationState {
            last_shown: Some("2026-07-31".into()),
            ..Default::default()
        };
        assert_eq!(due_donation(eastern(2026, 8, 1, 12, 0), &state), None);
    }

    #[test]
    fn anchored_to_eastern_not_the_users_clock() {
        // 08:00 Eastern on 2 Sep 2026 is 12:00 UTC.
        assert_eq!(key_of(eastern(2026, 9, 2, 8, 0)), Some("2026-09-02".into()));
        assert_eq!(key_of(Utc.with_ymd_and_hms(2026, 9, 2, 11, 59, 0).unwrap()), None);
        assert_eq!(key_of(Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap()), Some("2026-09-02".into()));
    }

    #[test]
    fn dst_is_handled() {
        // 8am Eastern is 13:00 UTC in July (EDT) and 12:00 UTC in November (EST).
        assert_eq!(key_of(eastern(2026, 7, 31, 10, 0)), Some("2026-07-31".into()));
        assert_eq!(
            key_of(Utc.with_ymd_and_hms(2026, 7, 31, 13, 59, 0).unwrap()),
            None
        );
        assert_eq!(
            key_of(Utc.with_ymd_and_hms(2026, 7, 31, 14, 0, 0).unwrap()),
            Some("2026-07-31".into())
        );
        // 25 Nov is EST: 10:00 Eastern is 15:00 UTC.
        assert_eq!(
            key_of(Utc.with_ymd_and_hms(2026, 11, 25, 15, 0, 0).unwrap()),
            Some("2026-11-25".into())
        );
    }
}
