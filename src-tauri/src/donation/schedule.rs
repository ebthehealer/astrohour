//! The donation reminder schedule, as data.
//!
//! The dates used to be constants in this module, which meant changing one
//! took a rebuild and a release. They now live in `donation-schedule.json`,
//! which is parsed at runtime.
//!
//! Two copies exist. One is compiled into the binary with `include_str!` and
//! is authoritative whenever nothing better is available — notably offline.
//! The other is a file on disk, which can be replaced by a fetch from
//! `astrohour.app` so the schedule can change without shipping a build.
//! `load` prefers the file and falls back to the bundled copy.
//!
//! A malformed file is never fatal: it is reported and the bundled copy is
//! used, so a typo on the server cannot silence the reminders.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use chrono::NaiveDate;
use chrono_tz::Tz;
use serde::Deserialize;

use super::When;

/// Compiled into the binary, and the fallback for everything.
const BUNDLED: &str = include_str!("../../donation-schedule.json");

/// The version this build understands. Bump it only for a breaking change;
/// an unrecognised version is rejected rather than guessed at.
const SUPPORTED_VERSION: u32 = 1;

/// One date, and how that date's moment is determined.
pub type Dated = ((u32, u32), When);

/// The reminder dates, per year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    default: Vec<Dated>,
    years: BTreeMap<i32, Vec<Dated>>,
}

impl Schedule {
    /// The dates for a year, in calendar order. Years with no entry of their
    /// own use `default`.
    pub fn for_year(&self, year: i32) -> &[Dated] {
        match self.years.get(&year) {
            Some(list) => list,
            None => &self.default,
        }
    }

    /// The copy shipped in the binary. Parsed once.
    pub fn bundled() -> &'static Self {
        static PARSED: OnceLock<Schedule> = OnceLock::new();
        PARSED.get_or_init(|| {
            Self::parse(BUNDLED).expect("the bundled donation schedule must parse")
        })
    }

    /// Parse a schedule file. Every failure is described well enough to act
    /// on, because the file is meant to be edited by hand.
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: File =
            serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;

        if file.version != SUPPORTED_VERSION {
            return Err(format!(
                "version {} is not supported by this build (it understands {SUPPORTED_VERSION})",
                file.version
            ));
        }

        let default = read_list(file.default)?;

        let mut years = BTreeMap::new();
        for (key, list) in file.years {
            let year: i32 = key
                .trim()
                .parse()
                .map_err(|_| format!("{key:?} is not a year"))?;
            years.insert(year, read_list(list)?);
        }

        Ok(Schedule { default, years })
    }
}

#[derive(Deserialize)]
struct File {
    version: u32,
    default: Vec<Entry>,
    #[serde(default)]
    years: BTreeMap<String, Vec<Entry>>,
}

/// A date is either bare — `"02-13"`, meaning the Jupiter hour — or spelled
/// out when it needs a pinned time.
#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Bare(String),
    Detailed {
        on: String,
        #[serde(default)]
        at: Option<String>,
    },
}

fn read_list(list: Vec<Entry>) -> Result<Vec<Dated>, String> {
    let mut out: Vec<Dated> = Vec::with_capacity(list.len());
    for entry in list {
        out.push(read_entry(entry)?);
    }
    out.sort_by_key(|((month, day), _)| (*month, *day));
    Ok(out)
}

fn read_entry(entry: Entry) -> Result<Dated, String> {
    let (on, at) = match entry {
        Entry::Bare(on) => (on, None),
        Entry::Detailed { on, at } => (on, at),
    };
    Ok((read_date(&on)?, read_when(at.as_deref())?))
}

/// `"02-13"` becomes `(2, 13)`. Validated against a leap year so that a
/// 29 February entry is accepted and 31 April is not.
fn read_date(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("{text:?} is not a date written as MM-DD");
    let (month, day) = text.trim().split_once('-').ok_or_else(bad)?;
    let month: u32 = month.parse().map_err(|_| bad())?;
    let day: u32 = day.parse().map_err(|_| bad())?;
    NaiveDate::from_ymd_opt(2000, month, day).ok_or_else(|| format!("{text:?} is not a real date"))?;
    Ok((month, day))
}

/// Either `None`/`"jupiter"`, or a time and an IANA zone such as
/// `"08:00 America/New_York"`.
fn read_when(text: Option<&str>) -> Result<When, String> {
    let Some(text) = text.map(str::trim) else {
        return Ok(When::FirstJupiterHour);
    };
    if text.is_empty() || text.eq_ignore_ascii_case("jupiter") {
        return Ok(When::FirstJupiterHour);
    }

    let (time, zone) = text.split_once(char::is_whitespace).ok_or_else(|| {
        format!("{text:?} needs a time and a zone, as in \"08:00 America/New_York\"")
    })?;

    let zone: Tz = zone
        .trim()
        .parse()
        .map_err(|_| format!("{:?} is not a time zone this build knows", zone.trim()))?;

    let (hours, minutes) = time
        .split_once(':')
        .ok_or_else(|| format!("{time:?} is not a time written as HH:MM"))?;
    let bad = || format!("{time:?} is not a time written as HH:MM");
    let hours: u32 = hours.parse().map_err(|_| bad())?;
    let minutes: u32 = minutes.parse().map_err(|_| bad())?;
    if hours > 23 || minutes > 59 {
        return Err(format!("{time:?} is not a time of day"));
    }

    Ok(When::At { hours, minutes, zone })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::America::New_York;

    /// A file with nothing but the two required fields.
    fn file(default: &str, years: &str) -> String {
        format!(r#"{{"version": 1, "default": {default}, "years": {years}}}"#)
    }

    fn parse(default: &str, years: &str) -> Result<Schedule, String> {
        Schedule::parse(&file(default, years))
    }

    // ── The two ways of writing a date ───────────────────────────────────────

    #[test]
    fn a_bare_date_means_the_jupiter_hour() {
        let s = parse(r#"["02-13"]"#, "{}").unwrap();
        assert_eq!(s.for_year(2027), &[((2, 13), When::FirstJupiterHour)]);
    }

    #[test]
    fn the_jupiter_hour_can_also_be_written_out() {
        let s = parse(r#"[{"on": "02-13", "at": "jupiter"}]"#, "{}").unwrap();
        assert_eq!(s.for_year(2027), &[((2, 13), When::FirstJupiterHour)]);
    }

    #[test]
    fn a_spelled_out_entry_can_omit_at() {
        let s = parse(r#"[{"on": "02-13"}]"#, "{}").unwrap();
        assert_eq!(s.for_year(2027), &[((2, 13), When::FirstJupiterHour)]);
    }

    #[test]
    fn a_date_can_pin_a_time_and_a_zone() {
        let s = parse(r#"[{"on": "09-02", "at": "08:00 America/New_York"}]"#, "{}").unwrap();
        assert_eq!(
            s.for_year(2027),
            &[(
                (9, 2),
                When::At {
                    hours: 8,
                    minutes: 0,
                    zone: New_York
                }
            )]
        );
    }

    #[test]
    fn the_two_forms_can_be_mixed() {
        let s = parse(
            r#"["02-13", {"on": "09-02", "at": "08:00 America/New_York"}, "10-11"]"#,
            "{}",
        )
        .unwrap();
        assert_eq!(s.for_year(2027).len(), 3);
        assert_eq!(s.for_year(2027)[0].0, (2, 13));
        assert_eq!(s.for_year(2027)[2].0, (10, 11));
    }

    #[test]
    fn dates_come_back_in_calendar_order() {
        let s = parse(r#"["10-11", "02-13", "07-31"]"#, "{}").unwrap();
        let order: Vec<(u32, u32)> = s.for_year(2027).iter().map(|(d, _)| *d).collect();
        assert_eq!(order, vec![(2, 13), (7, 31), (10, 11)]);
    }

    // ── Years ────────────────────────────────────────────────────────────────

    #[test]
    fn a_year_replaces_the_default_entirely() {
        let s = parse(r#"["02-13", "03-19"]"#, r#"{"2027": ["07-31"]}"#).unwrap();
        assert_eq!(s.for_year(2027), &[((7, 31), When::FirstJupiterHour)]);
    }

    #[test]
    fn years_without_an_entry_use_the_default() {
        let s = parse(r#"["02-13"]"#, r#"{"2027": ["07-31"]}"#).unwrap();
        assert_eq!(s.for_year(2026), &[((2, 13), When::FirstJupiterHour)]);
        assert_eq!(s.for_year(2028), &[((2, 13), When::FirstJupiterHour)]);
    }

    #[test]
    fn an_empty_list_means_no_reminders() {
        let s = parse(r#"["02-13"]"#, r#"{"2027": []}"#).unwrap();
        assert!(s.for_year(2027).is_empty());
        // ...and only for that year.
        assert_eq!(s.for_year(2028).len(), 1);
    }

    #[test]
    fn the_year_keys_may_be_quoted_or_not() {
        // JSON requires the quotes, but people forget them; say so clearly.
        let err = Schedule::parse(r#"{"version": 1, "default": [], "years": {2027: []}}"#)
            .unwrap_err();
        assert!(err.contains("not valid JSON"), "got: {err}");
    }

    // ── Rejections, each with something actionable to say ────────────────────

    #[test]
    fn a_future_version_is_refused_rather_than_guessed_at() {
        let err = Schedule::parse(r#"{"version": 99, "default": [], "years": {}}"#).unwrap_err();
        assert!(err.contains("99") && err.contains("not supported"), "got: {err}");
    }

    #[test]
    fn malformed_json_is_reported_as_such() {
        let err = Schedule::parse("{not json").unwrap_err();
        assert!(err.contains("not valid JSON"), "got: {err}");
    }

    #[test]
    fn a_date_that_does_not_exist_is_refused() {
        let err = parse(r#"["04-31"]"#, "{}").unwrap_err();
        assert!(err.contains("not a real date"), "got: {err}");
    }

    #[test]
    fn a_leap_day_is_accepted() {
        assert!(parse(r#"["02-29"]"#, "{}").is_ok());
    }

    #[test]
    fn a_date_in_the_wrong_shape_is_refused() {
        for bad in [r#""2027-02-13""#, r#""0213""#, r#""February 13""#] {
            let err = parse(&format!("[{bad}]"), "{}").unwrap_err();
            assert!(err.contains("MM-DD"), "{bad} gave: {err}");
        }
    }

    #[test]
    fn an_unknown_zone_is_refused() {
        let err = parse(r#"[{"on": "09-02", "at": "08:00 Mars/Olympus"}]"#, "{}").unwrap_err();
        assert!(err.contains("time zone"), "got: {err}");
    }

    #[test]
    fn a_time_without_a_zone_is_refused() {
        // Guessing a zone would silently move the reminder.
        let err = parse(r#"[{"on": "09-02", "at": "08:00"}]"#, "{}").unwrap_err();
        assert!(err.contains("needs a time and a zone"), "got: {err}");
    }

    #[test]
    fn a_time_out_of_range_is_refused() {
        for bad in ["25:00 America/New_York", "08:70 America/New_York"] {
            let err = parse(
                &format!(r#"[{{"on": "09-02", "at": "{bad}"}}]"#),
                "{}",
            )
            .unwrap_err();
            assert!(err.contains("time of day"), "{bad} gave: {err}");
        }
    }

    #[test]
    fn a_malformed_time_is_refused() {
        let err = parse(r#"[{"on": "09-02", "at": "8am America/New_York"}]"#, "{}").unwrap_err();
        assert!(err.contains("HH:MM"), "got: {err}");
    }

    // ── The shipped file ─────────────────────────────────────────────────────

    #[test]
    fn the_bundled_file_parses_and_says_what_we_expect() {
        let s = Schedule::bundled();
        let dates: Vec<(u32, u32)> = s.for_year(2027).iter().map(|(d, _)| *d).collect();
        assert_eq!(
            dates,
            vec![(2, 13), (3, 19), (5, 6), (7, 31), (9, 2), (10, 11)]
        );
        assert_eq!(
            s.for_year(2027)[4].1,
            When::At {
                hours: 8,
                minutes: 0,
                zone: New_York
            }
        );
    }
}
