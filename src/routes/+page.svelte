<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    initPrefs, getDaySchedule, getCurrentHour, getMoonPhase,
    planetSymbol, phaseEmoji, phaseName, formatTime, minutesUntil, formatCoord,
    type DaySchedule, type PlanetaryHour, type MoonPhase, type DisplayMode,
  } from '$lib/api';
  import FirstRun from '$lib/FirstRun.svelte';
  import Settings from '$lib/Settings.svelte';

  // ── App-level state
  let ready = false;          // prefs loaded
  let locationSet = false;    // has a real location (not fallback default)
  let showSettings = false;

  // Astronomical data
  let schedule: DaySchedule | null = null;
  let currentHour: PlanetaryHour | null = null;
  let moon: MoonPhase | null = null;
  let minsRemaining = 0;

  // Current prefs (mirrored from backend for display)
  let lat = 40.7128;
  let lon = -74.0060;
  let displayMode: DisplayMode = 'Short';

  // Listeners / timers
  let unlisten: (() => void) | null = null;
  let unlistenFocus: (() => void) | null = null;
  let tickInterval: ReturnType<typeof setInterval> | null = null;

  // ── Lifecycle
  onMount(async () => {
    const prefs = await initPrefs();
    lat = prefs.lat;
    lon = prefs.lon;
    displayMode = prefs.display_mode as DisplayMode;
    locationSet = prefs.location_set;
    ready = true;

    if (locationSet) {
      await refresh();
    }
    // Listen for 30-second backend tick
    unlisten = await listen('astro:tick', refresh);
    tickInterval = setInterval(updateMins, 30_000);

    // Dismiss window when it loses focus (macOS popover / Windows tray behaviour)
    const appWindow = getCurrentWindow();
    unlistenFocus = await appWindow.onFocusChanged(({ payload: focused }) => {
      if (!focused && !showSettings) {
        appWindow.hide();
      }
    });
  });

  onDestroy(() => {
    unlisten?.();
    unlistenFocus?.();
    if (tickInterval) clearInterval(tickInterval);
  });

  // ── Data refresh
  async function refresh() {
    [schedule, currentHour, moon] = await Promise.all([
      getDaySchedule(),
      getCurrentHour(),
      getMoonPhase(),
    ]);
    updateMins();
  }

  function updateMins() {
    if (currentHour) minsRemaining = Math.round(minutesUntil(currentHour.end));
  }

  // ── First-run done
  async function onFirstRunDone(e: CustomEvent<{ lat: number; lon: number }>) {
    lat = e.detail.lat;
    lon = e.detail.lon;
    locationSet = true;
    await refresh();
  }

  // ── Settings saved
  async function onSettingsSaved(e: CustomEvent<{ lat: number; lon: number; displayMode: DisplayMode }>) {
    lat = e.detail.lat;
    lon = e.detail.lon;
    displayMode = e.detail.displayMode;
    showSettings = false;
    await refresh();
  }

  // ── Derived display values
  $: dayName = new Date().toLocaleDateString([], { weekday: 'long' });
  $: sunrise = schedule ? formatTime(schedule.sunrise) : '--:--';
  $: sunset  = schedule ? formatTime(schedule.sunset)  : '--:--';
  $: dayRuler = schedule?.planetary_day ?? '';
</script>

{#if !ready}
  <!-- Blank while loading prefs — avoids flash -->
  <div class="splash">♃</div>

{:else if !locationSet}
  <FirstRun on:done={onFirstRunDone} />

{:else if showSettings}
  <Settings
    {lat} {lon} {displayMode}
    on:close={() => showSettings = false}
    on:saved={onSettingsSaved}
  />

{:else}
  <main>
    {#if !schedule || !currentHour || !moon}
      <div class="loading">
        <div class="spinner"></div>
      </div>
    {:else}

      <!-- ── Header: current hour -->
      <header class="current-hour">
        <span class="symbol">{planetSymbol(currentHour.planet)}</span>
        <div class="hour-info">
          <div class="hour-name">
            {currentHour.planet} Hour
            <span class="day-night">{currentHour.is_day ? 'Day' : 'Night'} {currentHour.sequence} of 12</span>
          </div>
          <div class="hour-times">
            {formatTime(currentHour.start)} → {formatTime(currentHour.end)}
            <span class="mins">{minsRemaining}m left</span>
          </div>
        </div>
        <button class="gear" on:click={() => showSettings = true} title="Settings">⚙️</button>
      </header>

      <!-- ── Day ruler + sun + moon -->
      <section class="day-moon">
        <div class="day-row">
          <span class="day-label">{dayName}</span>
          <span class="ruler">{planetSymbol(dayRuler)} {dayRuler} Day</span>
          <span class="sun-times">☀️ {sunrise} – {sunset}</span>
        </div>
        <div class="moon-row">
          <span class="phase-emoji">{phaseEmoji(moon.phase)}</span>
          <div class="moon-text">
            <div class="phase-name">{phaseName(moon.phase)}</div>
            <div class="lunar-day">Moon Day {moon.lunar_day} &middot; {Math.round(moon.illumination * 100)}% illuminated</div>
          </div>
        </div>
      </section>

      <!-- ── 24-hour list -->
      <section class="all-hours">
        <div class="hours-label">ALL 24 HOURS TODAY</div>
        <ul class="hours-list">
          {#each schedule.hours as h (h.index)}
            {@const isCurrent = h.index === currentHour.index}
            {@const isPast = !isCurrent && new Date(h.end) < new Date()}
            <li class="hour-row"
              class:current={isCurrent}
              class:night={!h.is_day}
              class:past={isPast}>
              <span class="h-sym">{planetSymbol(h.planet)}</span>
              <span class="h-time">{formatTime(h.start)}</span>
              <span class="h-name">{h.planet}</span>
              <span class="h-seq">{h.is_day ? '☀' : '🌙'} {h.sequence}</span>
              {#if isCurrent}<span class="h-now">◄ now</span>{/if}
            </li>
          {/each}
        </ul>
      </section>

      <!-- ── Footer: location -->
      <footer class="loc-footer">
        <span>📍 {formatCoord(lat, 'N', 'S')}, {formatCoord(lon, 'E', 'W')}</span>
        <button class="settings-link" on:click={() => showSettings = true}>Settings</button>
      </footer>

    {/if}
  </main>
{/if}

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px;
    background: #1a1a2e;
    color: #e0e0e0;
    overflow-x: hidden;
  }

  /* Splash */
  .splash {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    font-size: 48px;
    color: #e0b86a;
    opacity: 0.3;
  }

  /* Loading spinner */
  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
  }
  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid #223;
    border-top-color: #e0b86a;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  main {
    width: 360px;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }

  /* ── Current hour header */
  .current-hour {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 14px 14px 16px;
    background: #16213e;
    border-bottom: 1px solid #0f3460;
  }
  .symbol {
    font-size: 36px;
    color: #e0b86a;
    line-height: 1;
    flex-shrink: 0;
  }
  .hour-info { flex: 1; min-width: 0; }
  .hour-name {
    font-size: 15px;
    font-weight: 600;
    color: #fff;
  }
  .day-night {
    font-size: 10px;
    font-weight: 400;
    color: #888;
    margin-left: 6px;
  }
  .hour-times {
    font-size: 12px;
    color: #aaa;
    margin-top: 3px;
  }
  .mins { color: #e0b86a; margin-left: 8px; font-weight: 600; }
  .gear {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    padding: 4px;
    opacity: 0.5;
    flex-shrink: 0;
  }
  .gear:hover { opacity: 1; }

  /* ── Day + moon */
  .day-moon {
    padding: 10px 16px;
    background: #0f3460;
    border-bottom: 1px solid #1a4a80;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .day-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .day-label { color: #aaa; }
  .ruler { color: #e0b86a; font-weight: 600; }
  .sun-times { color: #aaa; font-size: 11px; margin-left: auto; }
  .moon-row { display: flex; align-items: center; gap: 10px; }
  .phase-emoji { font-size: 20px; }
  .phase-name { font-weight: 600; color: #fff; font-size: 13px; }
  .lunar-day { font-size: 11px; color: #aaa; }

  /* ── Hours list */
  .all-hours { flex: 1; overflow-y: auto; }
  .hours-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #445;
    padding: 9px 16px 3px;
    text-transform: uppercase;
  }
  .hours-list { list-style: none; margin: 0; padding: 0; }
  .hour-row {
    display: grid;
    grid-template-columns: 20px 44px 1fr 28px auto;
    align-items: center;
    gap: 7px;
    padding: 5px 16px;
    border-bottom: 1px solid #1a1a2e;
  }
  .hour-row.current {
    background: #1a3a5c;
    border-left: 3px solid #e0b86a;
    padding-left: 13px;
  }
  .hour-row.night { opacity: 0.75; }
  .hour-row.past  { opacity: 0.35; }
  .h-sym  { color: #e0b86a; font-size: 14px; text-align: center; }
  .h-time { color: #777; font-size: 11px; font-variant-numeric: tabular-nums; }
  .h-name { color: #ccc; }
  .h-seq  { color: #445; font-size: 11px; text-align: right; }
  .h-now  { color: #e0b86a; font-size: 10px; font-weight: 700; }

  /* ── Footer */
  .loc-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 16px;
    background: #12192e;
    border-top: 1px solid #1a3055;
    font-size: 11px;
    color: #556;
  }
  .settings-link {
    background: none;
    border: none;
    color: #556;
    font-size: 11px;
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
  }
  .settings-link:hover { color: #e0b86a; }
</style>
