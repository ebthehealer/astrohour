# Changelog

## 0.2.0 (2025-07-22)

### Added
- **Hour reminders** — optional 5/10/15/20/30-minute heads-up before a
  planetary hour ends, showing how long is left
- **Notification sounds** — pick a sound for hour changes and for reminders,
  with a preview button. Chime, bell, harp and singing bowl are synthesized
  in-process; "system default" defers to the desktop's own notification sound

### Fixed
- Tray hover card repeated the planet and hour name; the heading above it
  already carried both
- Sound picker labels were unreadable: WebKitGTK painted the selected value
  with the system theme's text colour instead of ours
- Tray tooltip and the hour-change timer used the system timezone while the
  window used the offset reported by the frontend, so they could disagree.
  Both now share one helper.

## 0.1.0 (2025-07-21)

### Added
- Planetary hours display in system status bar (tray icon)
- Current planetary hour with planet glyph and minutes remaining
- Full 24-hour day schedule
- Moon phase and lunar day
- Hour-change system notifications
- Notes per planetary hour
- Notes history
- 5 color themes
- 12-hour and 24-hour clock format setting
- Tray icon color picker (Auto, Accent, White, Gold, Grey)
- Status bar display modes (Icon only, Short, Full)
- Location detection (GPS / IP fallback)
- Launch at login (autostart)
- Linux: all features free
