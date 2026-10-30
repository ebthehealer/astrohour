// Temporary probe: when is the first Jupiter day hour on each donation date?
//
// Times are printed in US Eastern because that is the owner's zone; on a
// machine in another zone the instants are the same, the labels differ.
use astrohour_core::types::Planet;
use astrohour_core::{planetary_hours, Location};
use chrono::{Datelike, Local, NaiveDate};
use chrono_tz::America::New_York;

fn main() {
    let loc = Location { lat: 42.4604, lon: -71.3490 }; // the owner's location
    let dates = [(2, 13), (3, 19), (5, 6), (7, 31), (9, 2), (10, 11), (11, 25)];
    println!("today is {}\n", Local::now().format("%Y-%m-%d %A"));
    for year in [2027, 2028] {
        println!("--- {year} ---");
        for (m, d) in dates {
            let date = NaiveDate::from_ymd_opt(year, m, d).unwrap();
            let hours = planetary_hours(date, loc);
            match hours.iter().find(|h| h.is_day && h.planet == Planet::Jupiter) {
                Some(h) => println!(
                    "{:04}-{:02}-{:02} {:<9} hour {:>2}/12   {} - {}",
                    year, m, d, date.weekday(), h.sequence,
                    h.start.with_timezone(&New_York).format("%-I:%M %p"),
                    h.end.with_timezone(&New_York).format("%-I:%M %p"),
                ),
                None => println!("{year}-{m:02}-{d:02}  NO Jupiter day hour"),
            }
        }
    }
}
