//! Donation reminders.
//!
//! AstroHour is free on every platform. Seven times a year the app puts up a
//! donation card asking the user to support it. Each date carries a time of
//! day, and the card is shown once per occurrence.
//!
//! The decision is a pure function of the clock so it can be tested without
//! sleeping until September.

use chrono::{DateTime, Datelike, FixedOffset, Timelike};

/// The seven days, recurring every year.
///
/// Order is the order they were chosen in, which is not calendar order — the
/// year runs Feb 13, Mar 19, May 6, Jul 31, Sep 2, Oct 11, Nov 25.
/// Gaps are 34, 48, 86, 33, 39, 45 and 80 days.
///
/// Keep this sorted the way a human reads a year if it ever becomes editable;
/// `due_donation` does not care about order.
pub const DONATION_DATES: &[(u32, u32)] = &[
    (2, 13),
    (3, 19),
    (5, 6),
    (7, 31),
    (9, 2),
    (10, 11),
    (11, 25),
];

/// Time of day the card appears, in the user's local time, when no per-date
/// override is set. Mid-morning on purpose: late enough that the user has
/// opened their machine, early enough that it is not a disturbance.
pub const DEFAULT_TIME: (u32, u32) = (10, 0);

/// Is this a donation date?
pub fn is_donation_date(month: u32, day: u32) -> bool {
    DONATION_DATES.iter().any(|&(m, d)| m == month && d == day)
}

/// Stable identifier for one occurrence of a donation date, e.g. "2026-09-02".
/// Used to remember that we already asked this year.
pub fn occurrence_key(now: DateTime<FixedOffset>) -> String {
    format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
}

/// The time of day the card should appear on a given date, honouring any
/// per-date override and falling back to `DEFAULT_TIME`.
pub fn time_for_date(month: u32, day: u32, overrides: &[((u32, u32), (u32, u32))]) -> (u32, u32) {
    overrides
        .iter()
        .find(|((m, d), _)| *m == month && *d == day)
        .map(|(_, t)| *t)
        .unwrap_or(DEFAULT_TIME)
}

/// Should the donation card be shown right now?
///
/// Returns the occurrence key to record as shown, or `None`.
///
/// The card is due from the configured time until the end of that local day.
/// It is deliberately *not* due for a window of days afterwards: if the app
/// was closed all day on 2 September, asking about it on the 5th reads as a
/// debt rather than an invitation, and the next date is never far away.
///
/// Because the test is "now >= show_at" rather than "now == show_at", a user
/// who opens the app at 6pm on a donation date still sees it — the card is
/// driven by the first tick after the time, whenever that happens to be.
pub fn due_donation(
    now: DateTime<FixedOffset>,
    show_at: (u32, u32),
    last_shown: Option<&str>,
) -> Option<String> {
    if !is_donation_date(now.month(), now.day()) {
        return None;
    }
    if (now.hour(), now.minute()) < show_at {
        return None;
    }
    let key = occurrence_key(now);
    if last_shown == Some(key.as_str()) {
        return None;
    }
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> DateTime<FixedOffset> {
        FixedOffset::west_opt(4 * 3600)
            .unwrap()
            .with_ymd_and_hms(y, m, d, hh, mm, 0)
            .unwrap()
    }

    #[test]
    fn all_seven_dates_are_recognised() {
        for &(m, d) in DONATION_DATES {
            assert!(is_donation_date(m, d), "{m}-{d} should be a donation date");
        }
        assert_eq!(DONATION_DATES.len(), 7);
    }

    #[test]
    fn neighbouring_days_are_not_donation_dates() {
        assert!(!is_donation_date(9, 1));
        assert!(!is_donation_date(9, 3));
        assert!(!is_donation_date(12, 25));
    }

    #[test]
    fn not_due_before_the_configured_time() {
        assert_eq!(due_donation(at(2026, 9, 2, 9, 59), (10, 0), None), None);
    }

    #[test]
    fn due_at_and_after_the_configured_time() {
        assert_eq!(
            due_donation(at(2026, 9, 2, 10, 0), (10, 0), None),
            Some("2026-09-02".into())
        );
        // Opened late in the day — still shown.
        assert_eq!(
            due_donation(at(2026, 9, 2, 22, 30), (10, 0), None),
            Some("2026-09-02".into())
        );
    }

    #[test]
    fn not_due_on_an_ordinary_day() {
        assert_eq!(due_donation(at(2026, 9, 3, 12, 0), (10, 0), None), None);
    }

    #[test]
    fn shown_once_per_occurrence() {
        let seen = "2026-09-02";
        assert_eq!(due_donation(at(2026, 9, 2, 11, 0), (10, 0), Some(seen)), None);
    }

    #[test]
    fn last_years_record_does_not_suppress_this_year() {
        assert_eq!(
            due_donation(at(2027, 9, 2, 11, 0), (10, 0), Some("2026-09-02")),
            Some("2027-09-02".into())
        );
    }

    #[test]
    fn per_date_override_wins() {
        let overrides = [((11, 25), (19, 30))];
        assert_eq!(time_for_date(11, 25, &overrides), (19, 30));
        assert_eq!(time_for_date(9, 2, &overrides), DEFAULT_TIME);
    }

    #[test]
    fn every_date_gets_a_time() {
        for &(m, d) in DONATION_DATES {
            let t = time_for_date(m, d, &[]);
            assert!(t.0 < 24 && t.1 < 60, "{m}-{d} has an invalid time {t:?}");
        }
    }
}
