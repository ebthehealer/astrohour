/**
 * Typed wrappers around the Tauri IPC commands exposed by astrohour-core.
 * All types mirror the Rust structs (serde-serialized).
 */
import { invoke } from '@tauri-apps/api/core';

export interface Location {
  lat: number;
  lon: number;
}

export type DisplayMode = 'IconOnly' | 'Short' | 'Full';

export interface Planet {
  // Rust enum serialised as string by serde
  name: 'Sun' | 'Moon' | 'Mars' | 'Mercury' | 'Jupiter' | 'Venus' | 'Saturn';
}

export interface PlanetaryHour {
  index: number;        // 0-23
  planet: string;       // "Jupiter" etc.
  start: string;        // RFC3339
  end: string;          // RFC3339
  is_day: boolean;
  sequence: number;     // 1-12
}

export interface MoonPhase {
  phase: string;        // "FullMoon" etc.
  illumination: number; // 0.0-1.0
  lunar_day: number;    // 1-30
  elongation_deg: number;
}

export interface DaySchedule {
  date: string;         // "2025-01-09"
  location: Location;
  planetary_day: string;
  sunrise: string;      // RFC3339
  sunset: string;
  hours: PlanetaryHour[];
  moon: MoonPhase;
}

// Planet display helpers (matches Rust Planet::symbol / name)
const SYMBOLS: Record<string, string> = {
  Sun: '\u2609', Moon: '\u263D', Mars: '\u2642',
  Mercury: '\u263F', Jupiter: '\u2643', Venus: '\u2640', Saturn: '\u2644',
};
const PHASE_EMOJI: Record<string, string> = {
  NewMoon: '\uD83C\uDF11',
  WaxingCrescent: '\uD83C\uDF12',
  FirstQuarter: '\uD83C\uDF13',
  WaxingGibbous: '\uD83C\uDF14',
  FullMoon: '\uD83C\uDF15',
  WaningGibbous: '\uD83C\uDF16',
  LastQuarter: '\uD83C\uDF17',
  WaningCrescent: '\uD83C\uDF18',
};
const PHASE_NAME: Record<string, string> = {
  NewMoon: 'New Moon', WaxingCrescent: 'Waxing Crescent',
  FirstQuarter: 'First Quarter', WaxingGibbous: 'Waxing Gibbous',
  FullMoon: 'Full Moon', WaningGibbous: 'Waning Gibbous',
  LastQuarter: 'Last Quarter', WaningCrescent: 'Waning Crescent',
};

export const planetSymbol = (planet: string) => SYMBOLS[planet] ?? '?';
export const phaseEmoji   = (phase: string)  => PHASE_EMOJI[phase] ?? '?';
export const phaseName    = (phase: string)  => PHASE_NAME[phase]  ?? phase;

export function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

export function minutesUntil(iso: string): number {
  return Math.max(0, (new Date(iso).getTime() - Date.now()) / 60000);
}

// ── IPC calls ───────────────────────────────────────────────────────────

export const getCurrentHour  = ()           => invoke<PlanetaryHour>('get_current_hour');
export const getDaySchedule  = (date?: string) => invoke<DaySchedule>('get_day_schedule', { dateStr: date ?? null });
export const getStatusText   = ()           => invoke<string>('get_status_text');
export const getMoonPhase    = ()           => invoke<MoonPhase>('get_moon_phase');
export const getLocation     = ()           => invoke<Location>('get_location');
export const setLocation     = (lat: number, lon: number) => invoke<void>('set_location', { lat, lon });
export const getDisplayMode  = ()           => invoke<DisplayMode>('get_display_mode');
export const setDisplayMode  = (mode: DisplayMode) => invoke<void>('set_display_mode', { mode });
