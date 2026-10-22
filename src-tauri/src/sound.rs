//! Notification sounds — synthesized in pure Rust, played through whatever
//! audio player the host provides.
//!
//! We generate our own WAVs instead of shipping audio assets so the sound set
//! is identical on every platform, the installer stays small, and there are no
//! licensing questions about bundled clips. Playback shells out to the first
//! player found on PATH (`pw-play`, `paplay`, `canberra-gtk-play`, `aplay`,
//! `ffplay`, `afplay`, …) — every desktop has at least one of them.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Sample rate for all generated tones.
const SR: u32 = 44_100;

/// Selectable sound names.
///
/// `"system"` defers to the notification daemon's own theme sound (the
/// freedesktop `sound-name` hint). `"none"` is silent.
pub const SOUND_NAMES: &[&str] = &["system", "chime", "bell", "harp", "bowl", "none"];

pub fn is_valid_sound(name: &str) -> bool {
    SOUND_NAMES.contains(&name)
}

// ── Tone synthesis ────────────────────────────────────────────────────────────

struct Note {
    freq: f64,
    start: f64,
    dur: f64,
    amp: f64,
}

/// The score for each built-in sound: a list of notes plus total length in
/// seconds. Returns `None` for `"none"` / `"system"` (nothing to synthesize).
fn score(name: &str) -> Option<(Vec<Note>, f64)> {
    let n = |freq, start, dur, amp| Note { freq, start, dur, amp };
    match name {
        // Two rising bell tones — bright and friendly.
        "chime" => Some((
            vec![
                n(880.00, 0.00, 1.8, 0.9),  // A5
                n(1318.51, 0.16, 1.8, 0.7), // E6
            ],
            2.0,
        )),
        // A single struck bell with a slow decay.
        "bell" => Some((
            vec![
                n(523.25, 0.00, 2.8, 1.0),   // C5
                n(1046.50, 0.00, 1.6, 0.35), // C6
            ],
            3.0,
        )),
        // Quick ascending arpeggio, C major.
        "harp" => Some((
            vec![
                n(523.25, 0.00, 1.6, 0.8),  // C5
                n(659.25, 0.09, 1.6, 0.8),  // E5
                n(783.99, 0.18, 1.6, 0.8),  // G5
                n(1046.50, 0.27, 2.0, 0.9), // C6
            ],
            2.4,
        )),
        // Soft singing bowl — long, calm decay.
        "bowl" => Some((
            vec![
                n(220.00, 0.00, 3.6, 1.0),  // A3
                n(329.63, 0.04, 3.4, 0.40), // E4
                n(440.00, 0.08, 3.2, 0.25), // A4
            ],
            4.0,
        )),
        _ => None,
    }
}

/// Mix the notes down to a normalized mono buffer.
fn render(notes: &[Note], total: f64) -> Vec<f32> {
    let len = (total * SR as f64) as usize;
    let mut buf = vec![0.0f32; len];

    for note in notes {
        let offset = (note.start * SR as f64) as usize;
        let note_len = (note.dur * SR as f64) as usize;

        for i in 0..note_len {
            let idx = offset + i;
            if idx >= len {
                break;
            }
            let t = i as f64 / SR as f64;

            // Exponential decay to ~5% at the end of the note, with a very
            // short attack ramp so there is no click at onset.
            let decay = (-t * 3.0 / note.dur).exp();
            let attack = 1.0 - (-t * 200.0).exp();
            let env = decay * attack;

            // Slightly inharmonic partials give a struck-metal character.
            let phase = std::f64::consts::TAU * note.freq * t;
            let v = (phase).sin()
                + 0.45 * (phase * 2.01).sin()
                + 0.22 * (phase * 2.98).sin()
                + 0.12 * (phase * 4.20).sin();

            buf[idx] += (v * env * note.amp) as f32;
        }
    }

    // Normalize so every sound has the same perceived loudness.
    let peak = buf.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
    if peak > 0.0 {
        let k = 0.85 / peak;
        for s in buf.iter_mut() {
            *s *= k;
        }
    }
    buf
}

/// Encode a mono float buffer as 16-bit PCM WAV.
fn encode_wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);

    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SR.to_le_bytes());
    out.extend_from_slice(&(SR * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());

    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Path of the cached WAV for a sound.
fn cache_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join("astrohour-sounds")
        .join(format!("{name}.wav"))
}

/// Return the WAV path for a sound, synthesizing it on first use.
fn ensure_wav(name: &str) -> Option<PathBuf> {
    let path = cache_path(name);
    if path.is_file() {
        return Some(path);
    }

    let (notes, total) = score(name)?;
    let wav = encode_wav(&render(&notes, total));

    std::fs::create_dir_all(path.parent()?).ok()?;

    // Write to a temp name then rename, so a second caller racing us never
    // reads a half-written file.
    let tmp = path.with_extension("wav.tmp");
    {
        let mut f = std::fs::File::create(&tmp).ok()?;
        f.write_all(&wav).ok()?;
        f.sync_all().ok()?;
    }
    std::fs::rename(&tmp, &path).ok()?;

    Some(path)
}

// ── Playback ──────────────────────────────────────────────────────────────────

struct Player {
    cmd: &'static str,
    args: &'static [&'static str],
}

#[cfg(target_os = "linux")]
const PLAYERS: &[Player] = &[
    Player { cmd: "pw-play", args: &[] },
    Player { cmd: "paplay", args: &[] },
    Player { cmd: "canberra-gtk-play", args: &["-f"] },
    Player { cmd: "aplay", args: &["-q"] },
    Player { cmd: "ffplay", args: &["-nodisp", "-autoexit", "-loglevel", "quiet"] },
];

#[cfg(target_os = "macos")]
const PLAYERS: &[Player] = &[Player { cmd: "afplay", args: &[] }];

#[cfg(target_os = "windows")]
const PLAYERS: &[Player] = &[Player {
    cmd: "powershell",
    args: &["-NoProfile", "-Command"],
}];

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
const PLAYERS: &[Player] = &[];

/// Is `cmd` an executable somewhere on PATH?
fn on_path(cmd: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| {
        if dir.join(cmd).is_file() {
            return true;
        }
        cfg!(windows) && dir.join(format!("{cmd}.exe")).is_file()
    })
}

/// The first audio player present on this host, if any.
fn pick_player() -> Option<&'static Player> {
    PLAYERS.iter().find(|p| on_path(p.cmd))
}

/// Whether this host can play our synthesized sounds at all.
///
/// The UI uses this to explain why the sound picker is inert on a bare
/// system with no audio tooling installed.
pub fn available() -> bool {
    pick_player().is_some()
}

/// Play a sound by name. Returns `true` if playback was started.
///
/// `"none"` and `"system"` are no-ops here — `"system"` is handled by the
/// notification daemon via the `sound-name` hint instead.
pub fn play(name: &str) -> bool {
    if name == "none" || name == "system" {
        return false;
    }

    let Some(path) = ensure_wav(name) else {
        return false;
    };
    let Some(player) = pick_player() else {
        return false;
    };

    let mut cmd = Command::new(player.cmd);
    cmd.args(player.args);

    #[cfg(windows)]
    cmd.arg(format!(
        "(New-Object Media.SoundPlayer '{}').PlaySync();",
        path.display()
    ));
    #[cfg(not(windows))]
    cmd.arg(&path);

    let spawned = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match spawned {
        Ok(mut child) => {
            // Reap the child on a detached thread; otherwise we accumulate
            // zombies on Unix, one per notification.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            true
        }
        Err(e) => {
            eprintln!("[sound] failed to spawn {}: {e}", player.cmd);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_offered_name_is_accepted() {
        for name in SOUND_NAMES {
            assert!(is_valid_sound(name), "{name} should be valid");
        }
        assert!(!is_valid_sound("kazoo"));
    }

    #[test]
    fn tones_render_within_full_scale() {
        for name in SOUND_NAMES {
            let Some((notes, total)) = score(name) else { continue };
            let buf = render(&notes, total);
            assert_eq!(buf.len(), (total * SR as f64) as usize, "{name} length");

            let peak = buf.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            assert!(peak > 0.0, "{name} is silent");
            assert!(peak <= 1.0, "{name} clips at {peak}");

            // Nothing should be left ringing after the buffer ends.
            assert!(buf[buf.len() - 1].abs() < 0.05, "{name} ends abruptly");
        }
    }

    #[test]
    fn wav_header_is_well_formed() {
        let wav = encode_wav(&[0.0, 1.0, -1.0, 0.0]);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 8);

        // RIFF size covers everything after the first 8 bytes.
        let riff_size = u32::from_le_bytes(wav[4..8].try_into().unwrap());
        assert_eq!(riff_size as usize, wav.len() - 8);

        // Full-scale samples round-trip.
        let first = i16::from_le_bytes(wav[44..46].try_into().unwrap());
        let second = i16::from_le_bytes(wav[46..48].try_into().unwrap());
        assert_eq!(first, 0);
        assert_eq!(second, 32767);
    }

    #[test]
    fn cache_paths_are_distinct_per_sound() {
        assert_ne!(cache_path("chime"), cache_path("bell"));
        assert!(cache_path("chime").to_string_lossy().ends_with("chime.wav"));
    }

    /// Writes every built-in sound to a directory so it can be auditioned.
    /// Run with: `cargo test -- --ignored --nocapture dump_sounds`
    #[test]
    #[ignore]
    fn dump_sounds() {
        let dir = std::path::PathBuf::from(std::env::var("DUMP_DIR").unwrap_or("/tmp".into()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in SOUND_NAMES {
            let Some((notes, total)) = score(name) else { continue };
            let wav = encode_wav(&render(&notes, total));
            let path = dir.join(format!("astrohour-{name}.wav"));
            std::fs::write(&path, &wav).unwrap();
            println!("{name}: {total}s -> {}", path.display());
        }
    }
}
