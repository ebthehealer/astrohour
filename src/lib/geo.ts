/**
 * Geolocation helper.
 *
 * Strategy:
 *   1. Try tauri-plugin-geolocation (uses OS location service).
 *   2. On failure, fall back to ip-api.com (no key needed, ~city accuracy).
 *   3. On failure, return null — caller shows the manual entry form.
 *
 * On desktop the OS location dialog appears once and the user can always
 * override in Settings.
 */
import {
  checkPermissions,
  requestPermissions,
  getCurrentPosition,
} from '@tauri-apps/plugin-geolocation';

export interface GeoResult {
  lat: number;
  lon: number;
  source: 'gps' | 'ip' | 'manual';
}

export async function detectLocation(): Promise<GeoResult | null> {
  // 1. Try OS geolocation
  try {
    let perm = await checkPermissions();
    if (perm.location === 'prompt' || perm.location === 'prompt-with-rationale') {
      perm = await requestPermissions(['location']);
    }
    if (perm.location === 'granted') {
      const pos = await getCurrentPosition({ timeout: 8000 });
      return {
        lat: pos.coords.latitude,
        lon: pos.coords.longitude,
        source: 'gps',
      };
    }
  } catch (_) {
    // fall through to IP fallback
  }

  // 2. IP geolocation fallback
  try {
    const res = await fetch('http://ip-api.com/json/?fields=lat,lon,status');
    const json = await res.json() as { status: string; lat: number; lon: number };
    if (json.status === 'success') {
      return { lat: json.lat, lon: json.lon, source: 'ip' };
    }
  } catch (_) {
    // fall through
  }

  return null;
}
