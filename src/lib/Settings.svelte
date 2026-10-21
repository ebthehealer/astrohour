<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import {
    setLocation, setDisplayMode, savePrefs,
    setAutostart, getAutostart,
    formatCoord,
    setNotificationsEnabled,
    setClockFormat,
    type DisplayMode,
  } from './api';
  import { detectLocation } from './geo';
  import { themes, type Theme } from './themes';
  import { setTrayIconColor } from './api';

  export let lat: number;
  export let lon: number;
  export let displayMode: DisplayMode;
  export let currentTheme: string = 'midnight';
  export let trayIconColor: string = 'white';
  export let notificationsEnabled: boolean = true;
  export let clockFormat: string = '12';

  // 'auto' = white, letting the DE composite it (correct for SNI on dark panels).
  // 'accent' = resolved at render time from the current theme's --accent variable.
  const TRAY_COLORS = [
    { id: 'auto',    label: 'Auto',   hex: '#ffffff', note: 'Follows system panel' },
    { id: 'accent',  label: 'Accent', hex: '',        note: 'Current theme color'  },
    { id: 'white',   label: 'White',  hex: '#ffffff', note: '' },
    { id: 'gold',    label: 'Gold',   hex: '#c8a84b', note: '' },
    { id: '#aaaaaa', label: 'Grey',   hex: '#aaaaaa', note: '' },
  ];

  // Resolve the current theme's accent hex for live preview
  $: accentHex = getComputedStyle(document.documentElement)
    .getPropertyValue('--accent').trim() || '#c8a84b';

  const dispatch = createEventDispatcher<{
    close: void;
    saved: { lat: number; lon: number; displayMode: DisplayMode };
    themeChange: string;
    trayColorChange: string;
    notificationsChange: boolean;
    clockFormatChange: string;
  }>();

  // ── Local form state
  let latStr = lat.toFixed(6);
  let lonStr = lon.toFixed(6);
  let mode: DisplayMode = displayMode;
  let autostart = false;
  let geoStatus: 'idle' | 'loading' | 'ok' | 'error' = 'idle';
  let geoMsg = '';
  let saving = false;
  let saveError = '';

  // Load autostart state on mount
  getAutostart().then(v => { autostart = v; });

  async function useMyLocation() {
    geoStatus = 'loading';
    geoMsg = '';
    const result = await detectLocation();
    if (result) {
      latStr = result.lat.toFixed(6);
      lonStr = result.lon.toFixed(6);
      geoStatus = 'ok';
      geoMsg = result.source === 'gps'
        ? 'Got GPS location'
        : 'Got approximate location via IP';
    } else {
      geoStatus = 'error';
      geoMsg = 'Could not detect location. Enter manually below.';
    }
  }

  async function save() {
    saving = true;
    saveError = '';
    const newLat = parseFloat(latStr);
    const newLon = parseFloat(lonStr);
    if (isNaN(newLat) || newLat < -90 || newLat > 90) {
      saveError = 'Latitude must be −90 – +90';
      saving = false;
      return;
    }
    if (isNaN(newLon) || newLon < -180 || newLon > 180) {
      saveError = 'Longitude must be −180 – +180';
      saving = false;
      return;
    }
    try {
      await setLocation(newLat, newLon);
      await setDisplayMode(mode);
      await setAutostart(autostart);
      await setNotificationsEnabled(notificationsEnabled);
      await setClockFormat(clockFormat);
      await savePrefs();
      dispatch('saved', { lat: newLat, lon: newLon, displayMode: mode });
      dispatch('close');
    } catch (e) {
      saveError = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="settings-overlay" role="dialog" aria-modal="true">
  <div class="settings-panel">
    <div class="settings-header">
      <span>Settings</span>
      <button class="close-btn" on:click={() => dispatch('close')}>×</button>
    </div>

    <!-- Location -->
    <section>
      <div class="section-title">Location</div>

      <button class="geo-btn" on:click={useMyLocation} disabled={geoStatus === 'loading'}>
        {#if geoStatus === 'loading'}
          Detecting…
        {:else}
          📍 Use my location
        {/if}
      </button>

      {#if geoMsg}
        <div class="geo-msg" class:ok={geoStatus === 'ok'} class:err={geoStatus === 'error'}>
          {geoMsg}
        </div>
      {/if}

      <div class="field-row">
        <label for="lat">Latitude</label>
        <input id="lat" type="number" min="-90" max="90" step="0.0001"
          bind:value={latStr} placeholder="40.7128" />
      </div>
      <div class="field-row">
        <label for="lon">Longitude</label>
        <input id="lon" type="number" min="-180" max="180" step="0.0001"
          bind:value={lonStr} placeholder="-74.0060" />
      </div>

      {#if lat && lon}
        <div class="coord-display">
          Current: {formatCoord(lat, 'N', 'S')}, {formatCoord(lon, 'E', 'W')}
        </div>
      {/if}
    </section>

    <!-- Display mode -->
    <section>
      <div class="section-title">Status Bar Display</div>
      <div class="mode-group">
        {#each (['IconOnly', 'Short', 'Full'] as const) as m}
          <label class="mode-option" class:selected={mode === m}>
            <input type="radio" bind:group={mode} value={m} />
            <span class="mode-label">{m === 'IconOnly' ? '♃' : m === 'Short' ? '♃ Jupiter Hour' : '♃ Jupiter Hour · 38m'}</span>
            <span class="mode-name">{m}</span>
          </label>
        {/each}
      </div>
    </section>

    <!-- Tray icon color -->
    <section>
      <div class="section-title">Tray Icon Color</div>
      <div class="tray-color-row">
        {#each TRAY_COLORS as c}
          {@const dotColor = c.id === 'accent' ? accentHex : (c.hex || accentHex)}
          {@const borderColor = trayIconColor === c.id ? dotColor : 'transparent'}
          <button
            class="tray-color-btn"
            class:selected={trayIconColor === c.id}
            style="border-color:{borderColor};"
            on:click={async () => {
              const resolved = c.id === 'accent' ? accentHex
                             : c.id === 'auto'   ? 'auto'
                             : c.id;
              await setTrayIconColor(resolved);
              await savePrefs();
              dispatch('trayColorChange', c.id);
            }}
            title={c.note || c.label}
          >
            <span class="tray-dot" style="background:{dotColor}; box-shadow: 0 0 0 1px rgba(255,255,255,0.15);"></span>
            <span class="tray-label">{c.label}</span>
          </button>
        {/each}
      </div>
    </section>

    <!-- Theme -->
    <section>
      <div class="section-title">Color Theme</div>
      <div class="theme-grid">
        {#each Object.values(themes) as t}
          <button
            class="theme-swatch"
            class:selected={currentTheme === t.name}
            style="background:{t.bgBase}; border-color:{currentTheme === t.name ? t.accent : t.bgElevated};"
            on:click={() => dispatch('themeChange', t.name)}
            title={t.label}
          >
            <span style="color:{t.accent};">♃</span>
            <span style="color:{t.textPrimary}; font-size:10px;">{t.label}</span>
            <div class="swatch-dots">
              <div style="background:{t.accent};"></div>
              <div style="background:{t.planetNight};"></div>
              <div style="background:{t.bgElevated};"></div>
            </div>
          </button>
        {/each}
      </div>
    </section>

    <!-- Notifications -->
    <section>
      <div class="section-title">Notifications</div>
      <label class="toggle-row" for="notif-toggle">
        <span>Hour-change notifications</span>
        <div id="notif-toggle" class="toggle" class:on={notificationsEnabled}
          on:click={async () => {
            notificationsEnabled = !notificationsEnabled;
            await setNotificationsEnabled(notificationsEnabled);
            dispatch('notificationsChange', notificationsEnabled);
          }}
          role="switch" aria-checked={notificationsEnabled} tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && (notificationsEnabled = !notificationsEnabled)}>
          <div class="toggle-thumb"></div>
        </div>
      </label>
    </section>

    <!-- Clock format -->
    <section>
      <div class="section-title">Clock Format</div>
      <div class="clock-format-row">
        {#each [['12', '12-hour  (7:42 AM)'], ['24', '24-hour  (19:42)']] as [val, label]}
          <label class="mode-option" class:selected={clockFormat === val}>
            <input type="radio" bind:group={clockFormat} value={val}
              on:change={async () => {
                await setClockFormat(val);
                dispatch('clockFormatChange', val);
              }} />
            <span class="mode-label">{label}</span>
          </label>
        {/each}
      </div>
    </section>

    <!-- Autostart -->
    <section>
      <div class="section-title">System</div>
      <label class="toggle-row" for="autostart-toggle">
        <span>Launch at login</span>
        <div id="autostart-toggle" class="toggle" class:on={autostart} on:click={() => autostart = !autostart}
          role="switch" aria-checked={autostart} tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && (autostart = !autostart)}>
          <div class="toggle-thumb"></div>
        </div>
      </label>
    </section>

    {#if saveError}
      <div class="save-error">{saveError}</div>
    {/if}

    <div class="settings-footer">
      <button class="btn-cancel" on:click={() => dispatch('close')}>Cancel</button>
      <button class="btn-save" on:click={save} disabled={saving}>
        {saving ? 'Saving…' : 'Save'}
      </button>
    </div>
  </div>
</div>

<style>
  .settings-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.7);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    z-index: 100;
  }
  .settings-panel {
    background: var(--bg-surface, #16213e);
    width: 360px;
    max-height: 100vh;
    overflow-y: auto;
    border-radius: 0 0 10px 10px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.5);
  }
  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 16px;
    font-weight: 700;
    font-size: 14px;
    border-bottom: 1px solid var(--bg-elevated, #0f3460);
    color: var(--text-primary, #fff);
  }
  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #aaa);
    font-size: 20px;
    cursor: pointer;
    line-height: 1;
    padding: 0 4px;
  }
  .close-btn:hover { color: var(--text-primary, #fff); }

  section {
    padding: 14px 16px;
    border-bottom: 1px solid var(--bg-elevated, #0f3460);
  }
  .section-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted, #556);
    margin-bottom: 10px;
  }

  /* Geo button */
  .geo-btn {
    width: 100%;
    padding: 8px;
    background: var(--bg-elevated, #0f3460);
    border: 1px solid var(--bg-highlight, #1a4a80);
    border-radius: 6px;
    color: var(--accent, #e0b86a);
    font-size: 13px;
    cursor: pointer;
    margin-bottom: 8px;
  }
  .geo-btn:hover:not(:disabled) { background: var(--bg-highlight, #1a4a80); }
  .geo-btn:disabled { opacity: 0.5; cursor: wait; }
  .geo-msg {
    font-size: 11px;
    padding: 4px 6px;
    border-radius: 4px;
    margin-bottom: 8px;
  }
  .geo-msg.ok  { background: var(--bg-elevated); color: var(--success, #5dba7d); }
  .geo-msg.err { background: var(--bg-elevated); color: var(--danger, #e06060); }

  /* Coord inputs */
  .field-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .field-row label {
    width: 70px;
    font-size: 12px;
    color: var(--text-secondary, #aaa);
    flex-shrink: 0;
  }
  .field-row input {
    flex: 1;
    background: var(--bg-base, #0f1f3d);
    border: 1px solid var(--bg-highlight, #1a4a80);
    border-radius: 5px;
    padding: 6px 8px;
    color: var(--text-primary, #fff);
    font-size: 13px;
    outline: none;
  }
  .field-row input:focus { border-color: var(--accent, #e0b86a); }
  .coord-display {
    font-size: 11px;
    color: var(--text-muted, #556);
    margin-top: 4px;
  }

  /* Display mode */
  .mode-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .mode-option {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 6px;
    border: 1px solid var(--bg-elevated, #1a3055);
    cursor: pointer;
  }
  .mode-option.selected {
    border-color: var(--accent, #e0b86a);
    background: var(--bg-highlight, #1a3a5c);
  }
  .mode-option input[type=radio] { display: none; }
  .mode-label { font-size: 13px; color: var(--text-primary, #ddd); flex: 1; }
  .mode-name { font-size: 10px; color: var(--text-muted, #556); }

  /* Clock format */
  .clock-format-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  /* Toggle */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    color: var(--text-primary, #ddd);
    cursor: pointer;
    user-select: none;
  }
  .toggle {
    width: 40px;
    height: 22px;
    background: var(--bg-elevated, #1a3055);
    border-radius: 11px;
    position: relative;
    transition: background 0.2s;
    cursor: pointer;
  }
  .toggle.on { background: var(--accent, #e0b86a); }
  .toggle-thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    background: #fff;
    border-radius: 50%;
    transition: transform 0.2s;
  }
  .toggle.on .toggle-thumb { transform: translateX(18px); }

  /* Footer */
  .save-error {
    margin: 8px 16px 0;
    font-size: 12px;
    color: #e06060;
  }
  .settings-footer {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
    justify-content: flex-end;
  }
  .btn-cancel, .btn-save {
    padding: 7px 18px;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    border: none;
  }
  .btn-cancel { background: var(--bg-elevated, #0f3460); color: var(--text-secondary, #aaa); }
  .btn-cancel:hover { color: var(--text-primary, #fff); }
  .btn-save { background: var(--accent, #e0b86a); color: var(--text-inverse, #1a1a2e); font-weight: 700; }
  .btn-save:hover:not(:disabled) { filter: brightness(1.1); }
  .btn-save:disabled { opacity: 0.5; cursor: wait; }

  /* Tray color picker */
  .tray-color-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tray-color-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: 6px;
    border: 2px solid transparent;
    background: var(--bg-elevated);
    cursor: pointer;
    transition: border-color 0.15s;
    flex-shrink: 1;
  }
  .tray-color-btn.selected { background: var(--bg-highlight); }
  .tray-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }
  .tray-label {
    font-size: 11px;
    color: var(--text-primary);
    white-space: nowrap;
  }

  /* Theme swatches */
  .theme-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 4px 0;
  }
  .theme-swatch {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 8px 8px;
    border-radius: 8px;
    border: 2px solid transparent;
    cursor: pointer;
    font-size: 22px;
    transition: border-color 0.15s, transform 0.1s;
  }
  .theme-swatch:hover { transform: scale(1.03); }
  .theme-swatch.selected { box-shadow: 0 0 0 2px rgba(255,255,255,0.15); }
  .swatch-dots {
    display: flex;
    gap: 4px;
    margin-top: 2px;
  }
  .swatch-dots div {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
</style>
