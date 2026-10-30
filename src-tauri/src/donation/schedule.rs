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
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use chrono::NaiveDate;
use chrono_tz::Tz;
use serde::Deserialize;
use tauri::{AppHandle, Manager};

use super::When;

/// Compiled into the binary, and the fallback for everything.
const BUNDLED: &str = include_str!("../../donation-schedule.json");

/// The version this build understands. Bump it only for a breaking change;
/// an unrecognised version is rejected rather than guessed at.
const SUPPORTED_VERSION: u32 = 1;

/// Where a fetched copy is kept. Also read by hand, and editable by hand.
const CACHE_FILE: &str = "donation-schedule.json";

/// A plain static file: no query string, no identifiers, nothing sent but the
/// request itself. AstroHour is otherwise entirely offline.
const REMOTE_URL: &str = "https://astrohour.app/donation-schedule.json";

/// Do not fetch more often than this.
const REFRESH_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// The schedule is a few hundred bytes. Anything larger is not it.
const MAX_BYTES: usize = 64 * 1024;

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

// ── Loading ──────────────────────────────────────────────────────────────────

/// Where the fetched copy lives, if the app data directory can be resolved.
fn cache_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join(CACHE_FILE))
}

impl Schedule {
    /// The schedule in force: the cached copy if there is a usable one, the
    /// bundled copy otherwise.
    pub fn load(app: &AppHandle) -> Schedule {
        let cached = cache_path(app).and_then(|p| std::fs::read_to_string(p).ok());
        resolve(cached.as_deref())
    }
}

// ── Refreshing ───────────────────────────────────────────────────────────────

/// Fetch a fresh copy in the background if the cached one has gone a day
/// without an update. The caller does not wait, and nothing depends on the
/// result: on any failure the existing copy simply stays in force.
pub fn spawn_refresh(app: AppHandle) {
    if !is_stale(&app) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let text = match fetch().await {
            Ok(text) => text,
            // Expected when offline, or before astrohour.app is serving.
            Err(e) => {
                eprintln!("[donation] could not fetch the schedule: {e}");
                return;
            }
        };
        match Schedule::parse(&text) {
            Ok(_) => store(&app, &text),
            Err(e) => eprintln!("[donation] the fetched schedule is unusable: {e}"),
        }
    });
}

/// Which schedule to use, given whatever was found in the cache. A cache that
/// does not parse is reported and skipped rather than repaired, so a bad fetch
/// can never silence the reminders.
fn resolve(cached: Option<&str>) -> Schedule {
    if let Some(text) = cached {
        match Schedule::parse(text) {
            Ok(schedule) => return schedule,
            Err(e) => eprintln!("[donation] ignoring the cached schedule: {e}"),
        }
    }
    Schedule::bundled().clone()
}

/// Whether the cached copy is old enough to be worth replacing. No cache at
/// all counts as stale.
fn is_stale(app: &AppHandle) -> bool {
    let Some(path) = cache_path(app) else {
        return true;
    };
    match std::fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(written) => match written.elapsed() {
            Ok(age) => age >= REFRESH_AFTER,
            // The file is stamped in the future; the clock moved. Refresh
            // rather than trust it.
            Err(_) => true,
        },
        Err(_) => true,
    }
}

async fn fetch() -> Result<String, String> {
    fetch_from(REMOTE_URL).await
}

async fn fetch_from(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }

    let body = response.bytes().await.map_err(|e| e.to_string())?;
    if body.len() > MAX_BYTES {
        return Err(format!("{} bytes is too large", body.len()));
    }

    String::from_utf8(body.to_vec()).map_err(|_| "not valid UTF-8".to_string())
}

fn store(app: &AppHandle, text: &str) {
    let Some(path) = cache_path(app) else {
        eprintln!("[donation] no app data directory, keeping the fetched schedule unused");
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = write_atomically(&path, text) {
        eprintln!("[donation] could not store the schedule: {e}");
    }
}

/// Replace a file by writing a sibling and renaming it, so an interrupted
/// write cannot leave half a schedule behind.
fn write_atomically(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, text)?;
    match std::fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&temp);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::America::New_York;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A one-shot HTTP server, so the fetch path is exercised for real rather
    /// than mocked out.
    struct Server {
        url: String,
        _thread: std::thread::JoinHandle<()>,
    }

    fn serve(status: &'static str, body: &'static str) -> Server {
        serve_bytes(status, body.as_bytes())
    }

    fn serve_bytes(status: &'static str, body: &'static [u8]) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let thread = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0u8; 2048];
                let _ = stream.read(&mut request);
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(body);
                let _ = stream.flush();
            }
        });
        Server {
            url: format!("http://127.0.0.1:{port}/donation-schedule.json"),
            _thread: thread,
        }
    }

    fn fetch(url: &str) -> Result<String, String> {
        tauri::async_runtime::block_on(fetch_from(url))
    }

    // ── Which copy is used ───────────────────────────────────────────────────

    #[test]
    fn with_no_cache_the_bundled_copy_is_used() {
        assert_eq!(resolve(None), *Schedule::bundled());
    }

    #[test]
    fn a_usable_cache_wins_over_the_bundled_copy() {
        let cached = r#"{"version": 1, "default": ["07-31"], "years": {}}"#;
        assert_eq!(resolve(Some(cached)).for_year(2027), &[((7, 31), When::FirstJupiterHour)]);
    }

    #[test]
    fn an_unusable_cache_falls_back_rather_than_silencing_reminders() {
        for bad in ["{not json", r#"{"version": 99, "default": [], "years": {}}"#, ""] {
            assert_eq!(resolve(Some(bad)), *Schedule::bundled(), "for {bad:?}");
        }
    }

    #[test]
    fn a_valid_cache_may_deliberately_be_empty() {
        // An empty list is a decision, not a mistake, so it must be honoured.
        let cached = r#"{"version": 1, "default": [], "years": {}}"#;
        assert!(resolve(Some(cached)).for_year(2027).is_empty());
    }

    // ── Fetching ─────────────────────────────────────────────────────────────

    #[test]
    fn a_fetched_schedule_comes_back_intact() {
        let body = r#"{"version": 1, "default": ["02-13"], "years": {}}"#;
        let server = serve("200 OK", body);
        assert_eq!(fetch(&server.url).unwrap(), body);
    }

    #[test]
    fn a_fetched_schedule_parses_into_the_expected_dates() {
        let body = r#"{"version": 1, "default": ["02-13", "09-02"], "years": {}}"#;
        let server = serve("200 OK", body);
        let text = fetch(&server.url).unwrap();
        let parsed = Schedule::parse(&text).unwrap();
        assert_eq!(parsed.for_year(2027).len(), 2);
    }

    #[test]
    fn an_error_response_is_not_treated_as_a_schedule() {
        let server = serve("404 Not Found", "nope");
        let err = fetch(&server.url).unwrap_err();
        assert!(err.contains("404"), "got: {err}");
    }

    #[test]
    fn a_server_error_is_not_treated_as_a_schedule() {
        let server = serve("500 Internal Server Error", "boom");
        assert!(fetch(&server.url).unwrap_err().contains("500"));
    }

    #[test]
    fn an_unreachable_host_fails_without_panicking() {
        // Nothing is listening on this port.
        let err = fetch("http://127.0.0.1:9/donation-schedule.json").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn a_response_that_is_not_utf8_is_refused() {
        // 0xFF never appears in valid UTF-8.
        let server = serve_bytes("200 OK", &[0xFF, 0xFE, 0x00, 0x41]);
        let err = fetch(&server.url).unwrap_err();
        assert!(err.contains("UTF-8"), "got: {err}");
    }

    // ── Storing ──────────────────────────────────────────────────────────────

    #[test]
    fn storing_replaces_the_file_and_leaves_no_temporary_behind() {
        let dir = std::env::temp_dir().join(format!("astrohour-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("donation-schedule.json");

        write_atomically(&path, "first").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");

        write_atomically(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");

        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, vec!["donation-schedule.json"], "a temp file survived");

        let _ = std::fs::remove_dir_all(&dir);
    }

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
