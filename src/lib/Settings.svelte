<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import {
    setLocation, setDisplayMode, savePrefs,
    setAutostart, getAutostart,
    formatCoord,
    type DisplayMode,
  } from './api';
  import { detectLocation } from './geo';

  export let lat: number;
  export let lon: number;
  export let displayMode: DisplayMode;

  const dispatch = createEventDispatcher<{ close: void; saved: { lat: number; lon: number; displayMode: DisplayMode } }>();

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

    <!-- Autostart -->
    <section>
      <div class="section-title">System</div>
      <label class="toggle-row">
        <span>Launch at login</span>
        <div class="toggle" class:on={autostart} on:click={() => autostart = !autostart}
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
    background: #16213e;
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
    border-bottom: 1px solid #0f3460;
    color: #fff;
  }
  .close-btn {
    background: none;
    border: none;
    color: #aaa;
    font-size: 20px;
    cursor: pointer;
    line-height: 1;
    padding: 0 4px;
  }
  .close-btn:hover { color: #fff; }

  section {
    padding: 14px 16px;
    border-bottom: 1px solid #0f3460;
  }
  .section-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #556;
    margin-bottom: 10px;
  }

  /* Geo button */
  .geo-btn {
    width: 100%;
    padding: 8px;
    background: #0f3460;
    border: 1px solid #1a4a80;
    border-radius: 6px;
    color: #e0b86a;
    font-size: 13px;
    cursor: pointer;
    margin-bottom: 8px;
  }
  .geo-btn:hover:not(:disabled) { background: #1a4a80; }
  .geo-btn:disabled { opacity: 0.5; cursor: wait; }
  .geo-msg {
    font-size: 11px;
    padding: 4px 6px;
    border-radius: 4px;
    margin-bottom: 8px;
  }
  .geo-msg.ok  { background: #0d3320; color: #5dba7d; }
  .geo-msg.err { background: #3a1010; color: #e06060; }

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
    color: #aaa;
    flex-shrink: 0;
  }
  .field-row input {
    flex: 1;
    background: #0f1f3d;
    border: 1px solid #1a4a80;
    border-radius: 5px;
    padding: 6px 8px;
    color: #fff;
    font-size: 13px;
    outline: none;
  }
  .field-row input:focus { border-color: #e0b86a; }
  .coord-display {
    font-size: 11px;
    color: #556;
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
    border: 1px solid #1a3055;
    cursor: pointer;
  }
  .mode-option.selected {
    border-color: #e0b86a;
    background: #1a3a5c;
  }
  .mode-option input[type=radio] { display: none; }
  .mode-label { font-size: 13px; color: #ddd; flex: 1; }
  .mode-name { font-size: 10px; color: #556; }

  /* Toggle */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    color: #ddd;
    cursor: pointer;
    user-select: none;
  }
  .toggle {
    width: 40px;
    height: 22px;
    background: #1a3055;
    border-radius: 11px;
    position: relative;
    transition: background 0.2s;
    cursor: pointer;
  }
  .toggle.on { background: #e0b86a; }
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
  .btn-cancel { background: #0f3460; color: #aaa; }
  .btn-cancel:hover { color: #fff; }
  .btn-save { background: #e0b86a; color: #1a1a2e; font-weight: 700; }
  .btn-save:hover:not(:disabled) { background: #f0cc8a; }
  .btn-save:disabled { opacity: 0.5; cursor: wait; }
</style>
