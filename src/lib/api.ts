/**
 * Typed wrappers around the Tauri IPC commands exposed by astrohour-core.
 */
import { invoke } from '@tauri-apps/api/core';

export interface Location {
  lat: number;
  lon: number;
}

export interface LocationResult {
  lat: number;
  lon: number;
  location_set: boolean;
}

export interface InitPrefsResult {
  location_set: boolean;
  lat: number;
  lon: number;
  display_mode: DisplayMode;
}

export type DisplayMode = 'IconOnly' | 'Short' | 'Full';

export interface PlanetaryHour {
  index: number;
  planet: string;
  start: string;   // RFC3339
  end: string;     // RFC3339
  is_day: boolean;
  sequence: number; // 1-12
}

export interface MoonPhase {
  phase: string;
  illumination: number; // 0.0–1.0
  lunar_day: number;    // 1–30
  elongation_deg: number;
}

export interface DaySchedule {
  date: string;
  location: Location;
  planetary_day: string;
  sunrise: string;
  sunset: string;
  hours: PlanetaryHour[];
  moon: MoonPhase;
}

// ── Display helpers ────────────────────────────────────────────────────────────

const SYMBOLS: Record<string, string> = {
  Sun: '\u2609', Moon: '\u263D', Mars: '\u2642',
  Mercury: '\u263F', Jupiter: '\u2643', Venus: '\u2640', Saturn: '\u2644',
};
const PHASE_EMOJI: Record<string, string> = {
  NewMoon: '\uD83C\uDF11', WaxingCrescent: '\uD83C\uDF12',
  FirstQuarter: '\uD83C\uDF13', WaxingGibbous: '\uD83C\uDF14',
  FullMoon: '\uD83C\uDF15', WaningGibbous: '\uD83C\uDF16',
  LastQuarter: '\uD83C\uDF17', WaningCrescent: '\uD83C\uDF18',
};
const PHASE_NAME: Record<string, string> = {
  NewMoon: 'New Moon', WaxingCrescent: 'Waxing Crescent',
  FirstQuarter: 'First Quarter', WaxingGibbous: 'Waxing Gibbous',
  FullMoon: 'Full Moon', WaningGibbous: 'Waning Gibbous',
  LastQuarter: 'Last Quarter', WaningCrescent: 'Waning Crescent',
};

export const planetSymbol = (p: string) => SYMBOLS[p] ?? '?';
export const phaseEmoji   = (p: string) => PHASE_EMOJI[p] ?? '?';
export const phaseName    = (p: string) => PHASE_NAME[p]  ?? p;

export function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}
export function minutesUntil(iso: string): number {
  return Math.max(0, (new Date(iso).getTime() - Date.now()) / 60_000);
}
export function formatCoord(v: number, posLabel: string, negLabel: string): string {
  return `${Math.abs(v).toFixed(4)}° ${v >= 0 ? posLabel : negLabel}`;
}

// ── IPC calls ─────────────────────────────────────────────────────────────────

export const initPrefs      = ()                    => invoke<InitPrefsResult>('init_prefs');
export const savePrefs      = ()                    => invoke<void>('save_prefs');
export const getLocation    = ()                    => invoke<LocationResult>('get_location');
export const setLocation    = (lat: number, lon: number) => invoke<void>('set_location', { lat, lon });
export const getAutostart   = ()                    => invoke<boolean>('get_autostart');
export const setAutostart   = (enabled: boolean)   => invoke<void>('set_autostart', { enabled });
export const getCurrentHour = ()                    => invoke<PlanetaryHour>('get_current_hour');
export const getDaySchedule = (date?: string)       => invoke<DaySchedule>('get_day_schedule', { dateStr: date ?? null });
export const getStatusText  = ()                    => invoke<string>('get_status_text');
export const getMoonPhase   = ()                    => invoke<MoonPhase>('get_moon_phase');
export const getDisplayMode = ()                    => invoke<DisplayMode>('get_display_mode');
export const setDisplayMode = (mode: DisplayMode)   => invoke<void>('set_display_mode', { mode });

// ── Phase 2: Notes + License types ─────────────────────────────────────────────

export interface Note {
  id: number;
  planet_key: string;
  body: string;
  created_at: string;
  updated_at: string;
}

export type LicenseStatus =
  | { status: 'none' }
  | { status: 'active'; key_hash: string; activated_at: string }
  | { status: 'invalid' };

// ── Phase 2: Notes IPC ───────────────────────────────────────────────────

export const makePlanetKey = (date: string, hourIndex: number) =>
  invoke<string>('make_planet_key', { date, hourIndex });

export const todayString = () => invoke<string>('today_string');

// ── Phase 2: License IPC ─────────────────────────────────────────────────

export const getLicenseStatus  = ()                 => invoke<LicenseStatus>('get_license_status');
export const activateLicense   = (rawKey: string)   => invoke<string>('activate_license', { rawKey });
export const deactivateLicense = ()                 => invoke<void>('deactivate_license');
export const restoreLicense    = (keyHash: string | null, activatedAt: string | null) =>
  invoke<void>('restore_license', { keyHash, activatedAt });
