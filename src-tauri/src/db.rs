//! Database layer — SQLite via tauri-plugin-sql.
//!
//! The SQL plugin is initialised in lib.rs with the migrations defined here.
//! Actual queries run from the frontend via the JS `@tauri-apps/plugin-sql` bindings.
//! Rust-side commands in commands.rs provide helpers (make_planet_key, today_string).
//!
//! ## planet_key format
//! `"YYYY-MM-DD:N"` where N is the zero-based index into the day's
//! `DaySchedule::hours` vector.

use tauri_plugin_sql::{Migration, MigrationKind};

pub const DB_URL: &str = "sqlite:astrohour.db";

pub fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version:     1,
            description: "initial schema",
            sql:         include_str!("../migrations/0001_initial.sql"),
            kind:        MigrationKind::Up,
        },
    ]
}
