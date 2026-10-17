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
  import Notes from '$lib/Notes.svelte';
  import NotesHistory from '$lib/NotesHistory.svelte';
  import License from '$lib/License.svelte';
  import { makePlanetKey, todayString, getLicenseStatus, type LicenseStatus } from '$lib/api';
  import { themes, applyTheme, DEFAULT_THEME } from '$lib/themes';

  // ── App-level state
  let ready = false;          // prefs loaded
  let locationSet = false;    // has a real location (not fallback default)
  let showSettings = false;
  let showLicense  = false;
  let showNotesHistory = false;

  // Notes overlay state
  let notesKey:  string | null = null;  // planet_key being edited
  let notesName: string | null = null;  // human label for the overlay header

  // License status (refreshed on mount + after activation)
  let licenseStatus: LicenseStatus = { status: 'none' };
  $: isPremium   = licenseStatus.status === 'active';
  // On Linux all features are free; backend sets key_hash = 'linux-free'.
  $: isLinuxFree = isPremium && (licenseStatus as any).key_hash === 'linux-free';

  // Time-travel date (null = today)
  let travelDate: string | null = null;

  // Astronomical data
  let schedule: DaySchedule | null = null;
  let currentHour: PlanetaryHour | null = null;
  let moon: MoonPhase | null = null;
  let minsRemaining = 0;

  // Current prefs (mirrored from backend for display)
  let lat = 40.7128;
  let lon = -74.0060;
  let displayMode: DisplayMode = 'Short';
  let deviceSecret = '';
  let currentTheme = DEFAULT_THEME;

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
    deviceSecret = prefs.device_secret;
    // Apply saved theme (falls back to default if not set)
    const savedTheme = (prefs as any).theme ?? DEFAULT_THEME;
    currentTheme = savedTheme in themes ? savedTheme : DEFAULT_THEME;
    applyTheme(themes[currentTheme]);
    ready = true;

    if (locationSet) {
      await refresh();
    }
    // Load license status
    licenseStatus = await getLicenseStatus();
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
      getDaySchedule(travelDate ?? undefined),
      getCurrentHour(),
      getMoonPhase(),
    ]);
    updateMins();
  }

  // ── Notes
  async function openNotes(hourIndex: number, planetName: string) {
    if (!isPremium) { showLicense = true; return; }
    const today = await todayString();
    const date  = travelDate ?? today;
    notesKey  = await makePlanetKey(date, hourIndex);
    notesName = `${planetName} Hour`;
  }

  // ── Time travel (premium only)
  function prevDay() {
    if (!isPremium) { showLicense = true; return; }
    const base = travelDate ? new Date(travelDate) : new Date();
    base.setDate(base.getDate() - 1);
    travelDate = base.toISOString().slice(0, 10);
    refresh();
  }
  function nextDay() {
    if (!isPremium) { showLicense = true; return; }
    const base = travelDate ? new Date(travelDate) : new Date();
    base.setDate(base.getDate() + 1);
    travelDate = base.toISOString().slice(0, 10);
    refresh();
  }
  function resetToToday() {
    travelDate = null;
    refresh();
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

  // ── License activated callback
  async function onLicenseActivated() {
    licenseStatus = await getLicenseStatus();
    showLicense = false;
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

{:else if showNotesHistory}
  <NotesHistory
    onClose={() => showNotesHistory = false}
    onOpenNote={(key, name) => { showNotesHistory = false; notesKey = key; notesName = name; }}
  />

{:else if notesKey && notesName}
  <Notes
    planetKey={notesKey}
    planetName={notesName}
    onClose={() => { notesKey = null; notesName = null; }}
  />

{:else if showLicense}
  <License
    {deviceSecret}
    onClose={() => showLicense = false}
    onActivated={onLicenseActivated}
  />

{:else if showSettings}
  <Settings
    {lat} {lon} {displayMode} {currentTheme}
    on:close={() => showSettings = false}
    on:saved={onSettingsSaved}
    on:themeChange={(e) => {
      currentTheme = e.detail;
      applyTheme(themes[currentTheme]);
    }}
  />

{:else}
  <main>
    {#if !schedule || !currentHour || !moon}
      <div class="loading">
        <div class="spinner"></div>
      </div>
    {:else}

      <!-- ── Header: current hour (draggable title bar since decorations=false) -->
      <header class="current-hour">
        <div class="drag-region" data-tauri-drag-region>
          <span class="symbol">{planetSymbol(currentHour.planet)}</span>
          <div class="hour-info">
            {#if displayMode === 'IconOnly'}
              <div class="hour-name">
                {currentHour.planet} Hour
                <span class="day-night">{currentHour.is_day ? '☀' : '🌙'} {currentHour.sequence}/12</span>
              </div>
            {:else if displayMode === 'Short'}
              <div class="hour-name">
                {currentHour.planet} Hour
                <span class="day-night">{currentHour.is_day ? 'Day' : 'Night'} {currentHour.sequence} of 12</span>
              </div>
              <div class="hour-times">
                {formatTime(currentHour.start)} → {formatTime(currentHour.end)}
                <span class="mins">{minsRemaining}m left</span>
              </div>
            {:else}
              <div class="hour-name">
                {currentHour.planet} Hour
                <span class="day-night">{currentHour.is_day ? 'Day' : 'Night'} {currentHour.sequence} of 12</span>
              </div>
              <div class="hour-times">
                {formatTime(currentHour.start)} → {formatTime(currentHour.end)}
                <span class="mins">{minsRemaining}m left</span>
              </div>
              <div class="hour-extra">
                {phaseName(moon.phase)} · Moon Day {moon.lunar_day} · {Math.round(moon.illumination*100)}%
              </div>
            {/if}
          </div>
        </div>
        <button class="win-btn close" on:click={async () => { await getCurrentWindow().hide(); }} title="Close to tray">&#x2715;</button>
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

      <!-- ── Time travel bar (premium) -->
      <div class="time-travel">
        <button class="tt-btn" on:click={prevDay} title="Previous day">‹</button>
        <span class="tt-date" class:today={!travelDate}>
          {#if travelDate}
            {new Date(travelDate).toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' })}
            <button class="tt-today" on:click={resetToToday}>Today</button>
          {:else}
            Today
          {/if}
          {#if !isPremium}<span class="tt-lock">&#128274; Premium</span>{/if}
        </span>
        <button class="tt-btn" on:click={nextDay} title="Next day">›</button>
      </div>

      <!-- ── 24-hour list -->
      <section class="all-hours">
        <div class="hours-label">{travelDate ? 'HOURS FOR THIS DATE' : 'ALL 24 HOURS TODAY'}</div>
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
              <button class="h-note" on:click={() => openNotes(h.index, h.planet)}
                title="{isPremium ? 'Add note' : 'Premium: notes'}"
              >&#9998;</button>
            </li>
          {/each}
        </ul>
      </section>

      <!-- ── Footer: location + actions -->
      <footer class="loc-footer">
        <span>📍 {formatCoord(lat, 'N', 'S')}, {formatCoord(lon, 'E', 'W')}</span>
        <div class="footer-actions">
          {#if isLinuxFree}
            <a class="settings-link donation" href="https://astrohour.app/donate" target="_blank" rel="noreferrer">♥ Donate</a>
          {:else if !isPremium}
            <button class="settings-link premium" on:click={() => showLicense = true}>✨ Premium</button>
          {/if}
          <button class="settings-link" on:click={() => showNotesHistory = true}>📓 Notes</button>
          <button class="settings-link" on:click={() => showSettings = true}>⚙️ Settings</button>
        </div>
      </footer>

    {/if}
  </main>
{/if}

<style>
  /* ── CSS custom properties (set by applyTheme() at runtime) ── */
  :global(:root) {
    --bg-base:        #000000;
    --bg-surface:     #0a0a0a;
    --bg-elevated:    #141414;
    --bg-highlight:   #1e1e1e;
    --text-primary:   #f0f0f0;
    --text-secondary: #888888;
    --text-muted:     #444444;
    --text-inverse:   #000000;
    --accent:         #c8a84b;
    --accent-soft:    #3a2e10;
    --planet-day:     #c8a84b;
    --planet-night:   #6677bb;
    --danger:         #b06060;
    --success:        #4a9060;
    --btn-bg:         #141414;
    --btn-text:       #f0f0f0;
    --link-color:     #6655bb;
  }

  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px;
    background: var(--bg-base);
    color: var(--text-primary);
    overflow: hidden;
    height: 100vh;
    width: 100vw;
  }

  /* Splash */
  .splash {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    font-size: 48px;
    color: var(--accent);
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
    border: 2px solid var(--bg-elevated);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  main {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* ── Current hour header */
  .current-hour {
    display: flex;
    align-items: center;
    gap: 0;
    padding: 0;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--bg-elevated);
  }
  /* Drag region fills the header, contains symbol + info */
  .drag-region {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    padding: 14px 8px 14px 16px;
    cursor: grab;
  }
  .drag-region:active { cursor: grabbing; }
  [data-tauri-drag-region], .drag-region { -webkit-app-region: drag; }
  button, a, input, textarea, select { -webkit-app-region: no-drag; }
  .symbol {
    font-size: 36px;
    color: var(--accent);
    line-height: 1;
    flex-shrink: 0;
  }
  .hour-info { flex: 1; min-width: 0; }
  .hour-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .day-night {
    font-size: 10px;
    font-weight: 400;
    color: var(--text-secondary);
    margin-left: 6px;
  }
  .hour-times {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 3px;
  }
  .mins { color: var(--accent); margin-left: 8px; font-weight: 600; }
  .hour-extra {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 2px;
  }
  /* Close button (frameless window) */
  .win-btn.close {
    background: none;
    border: none;
    font-size: 15px;
    line-height: 1;
    padding: 6px 10px;
    cursor: pointer;
    color: var(--text-muted);
    flex-shrink: 0;
    align-self: stretch;
    display: flex;
    align-items: center;
  }
  .win-btn.close:hover { background: var(--danger); color: #fff; }

  /* ── Day + moon */
  .day-moon {
    padding: 10px 16px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--bg-highlight);
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
  .day-label { color: var(--text-secondary); }
  .ruler { color: var(--accent); font-weight: 600; }
  .sun-times { color: var(--text-secondary); font-size: 11px; margin-left: auto; }
  .moon-row { display: flex; align-items: center; gap: 10px; }
  .phase-emoji { font-size: 20px; }
  .phase-name { font-weight: 600; color: var(--text-primary); font-size: 13px; }
  .lunar-day { font-size: 11px; color: var(--text-secondary); }

  /* ── Hours list */
  .all-hours { flex: 1; overflow-y: auto; min-height: 0; }
  .hours-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    padding: 9px 16px 3px;
    text-transform: uppercase;
  }
  .hours-list { list-style: none; margin: 0; padding: 0; }
  .hour-row.current {
    background: var(--bg-highlight);
    border-left: 3px solid var(--accent);
    padding-left: 13px;
  }
  /* past hours: still readable */
  .hour-row.past  { opacity: 0.6; }
  /* night hours: symbol tinted, text unchanged */
  .h-sym  { color: var(--accent); font-size: 15px; text-align: center; }
  .hour-row.night .h-sym { color: var(--planet-night); }
  .h-time { color: var(--text-secondary); font-size: 12px; font-variant-numeric: tabular-nums; }
  .h-name { color: var(--text-primary); font-size: 13px; }
  .h-seq  { color: var(--text-secondary); font-size: 11px; text-align: right; }
  .h-now  { color: var(--accent); font-size: 10px; font-weight: 700; }

  /* ── Time travel bar */
  .time-travel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 5px 12px;
    background: var(--bg-base);
    border-bottom: 1px solid var(--bg-surface);
    font-size: 12px;
    color: var(--text-secondary);
  }
  .tt-btn {
    background: none;
    border: none;
    color: var(--link-color);
    font-size: 18px;
    cursor: pointer;
    padding: 0 6px;
    line-height: 1;
  }
  .tt-btn:hover { color: var(--accent); }
  .tt-date { flex: 1; text-align: center; }
  .tt-date.today { color: var(--text-muted); }
  .tt-today {
    background: none; border: none;
    color: var(--link-color); font-size: 11px;
    cursor: pointer; margin-left: 6px;
    text-decoration: underline;
  }
  .tt-lock { color: var(--text-muted); font-size: 10px; margin-left: 6px; }

  /* ── Hour row note button */
  .hour-row {
    display: grid;
    grid-template-columns: 20px 44px 1fr 28px auto auto;
    align-items: center;
    gap: 7px;
    padding: 5px 16px;
    border-bottom: 1px solid var(--bg-base);
  }
  .h-note {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 14px;
    cursor: pointer;
    padding: 2px 4px;
    opacity: 0.5;
    transition: opacity 0.15s, color 0.15s;
    border-radius: 3px;
  }
  .hour-row:hover .h-note { opacity: 0.85; }
  .h-note:hover { opacity: 1 !important; color: var(--accent); background: var(--bg-elevated); }
  /* Note present indicator */
  .h-note.has-note { color: var(--accent); opacity: 0.8; }

  /* ── Footer */
  .loc-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 16px;
    background: var(--bg-surface);
    border-top: 1px solid var(--bg-elevated);
    font-size: 11px;
    color: var(--text-secondary);
  }
  .footer-actions { display: flex; gap: 10px; align-items: center; }
  .settings-link {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    text-decoration: none;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .settings-link:hover { color: var(--text-primary); background: var(--bg-elevated); }
  .settings-link.premium { color: var(--link-color); }
  .settings-link.premium:hover { color: var(--accent); background: var(--bg-elevated); }
  .settings-link.donation { color: var(--danger); }
  .settings-link.donation:hover { color: var(--accent); background: var(--bg-elevated); }
</style>
