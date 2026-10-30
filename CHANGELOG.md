# Changelog

## 0.4.0 (2026-10-30)

### Added
- **The Moon's zodiac sign** is shown with the phase, for example
  "Waning Gibbous in Cancer ♋︎"
- **Tropical or sidereal.** Settings → Moon Sign Zodiac switches between the
  Western tropical zodiac and the Vedic sidereal zodiac (Lahiri ayanamsa)

### Fixed
- The day and moon header lines up properly: the date sits with the weekday,
  the sun and moon times sit on the right, and the moon block is centred
- Updating the installed app always embeds the current interface

## 0.3.0 (2026-10-30)

AstroHour is free on every platform. There is no licence, no account and
no advertising — it is supported by donations instead.

### Changed
- **The paywall is gone.** Notes and the time-travel arrows were locked
  behind a licence and are now simply available. The Premium button in
  the footer is now a Donate link for everyone, not just Linux users
- Reminders are asked for on six dates a year, at the start of the first
  Jupiter-ruled hour — derived from sunrise where you are, so the card
  arrives in your own morning rather than at one instant worldwide
- The donation card offers three answers: Donate, Not now (which waits
  for the next date), and Don't ask again

### Removed
- The licence system, its screen, and the Stronghold vault that held the
  key. Nothing else used the vault, and its files were shared between
  running copies of the app, which is what made a dev build and an
  installed build corrupt each other

### Fixed
- The weekday label froze at whatever day the app was launched on; it had
  no dependencies, so it was evaluated once and never again
- `rpm -q --quiet` returns empty rather than a version, which broke the
  install script's upgrade detection
- The tray hover card repeated the planet and hour name already shown in
  its heading

### Added
- Donation card, with the AstroHour mark and a Jupiter glyph on the
  donate button

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
