<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { setLocation, savePrefs } from './api';
  import { detectLocation } from './geo';

  const dispatch = createEventDispatcher<{ done: { lat: number; lon: number } }>();

  let status: 'detecting' | 'manual' | 'saving' = 'detecting';
  let error = '';
  let latStr = '';
  let lonStr = '';

  onMount(async () => {
    // Try auto-detect immediately
    const result = await detectLocation();
    if (result) {
      await setLocation(result.lat, result.lon);
      await savePrefs();
      dispatch('done', { lat: result.lat, lon: result.lon });
    } else {
      status = 'manual';
    }
  });

  async function saveManual() {
    const lat = parseFloat(latStr);
    const lon = parseFloat(lonStr);
    if (isNaN(lat) || lat < -90 || lat > 90) {
      error = 'Latitude must be −90 to +90';
      return;
    }
    if (isNaN(lon) || lon < -180 || lon > 180) {
      error = 'Longitude must be −180 to +180';
      return;
    }
    status = 'saving';
    error = '';
    try {
      await setLocation(lat, lon);
      await savePrefs();
      dispatch('done', { lat, lon });
    } catch (e) {
      error = String(e);
      status = 'manual';
    }
  }
</script>

<div class="first-run">
  <div class="logo">♃</div>
  <h1>AstroHour</h1>
  <p class="sub">Planetary hours for your location</p>

  {#if status === 'detecting'}
    <div class="detecting">
      <div class="spinner"></div>
      <span>Detecting your location…</span>
    </div>

  {:else if status === 'manual' || status === 'saving'}
    <p class="prompt">Enter your coordinates to calculate accurate planetary hours.</p>
    <p class="hint">You can find these at <strong>maps.google.com</strong> or use any city lookup.</p>

    <div class="field">
      <label for="fr-lat">Latitude</label>
      <input id="fr-lat" type="number" min="-90" max="90" step="0.0001"
        bind:value={latStr} placeholder="e.g. 40.7128" />
    </div>
    <div class="field">
      <label for="fr-lon">Longitude</label>
      <input id="fr-lon" type="number" min="-180" max="180" step="0.0001"
        bind:value={lonStr} placeholder="e.g. -74.0060" />
    </div>

    {#if error}
      <div class="err">{error}</div>
    {/if}

    <button class="save-btn" on:click={saveManual} disabled={status === 'saving'}>
      {status === 'saving' ? 'Saving…' : 'Set Location'}
    </button>
  {/if}
</div>

<style>
  .first-run {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 32px 24px;
    text-align: center;
    background: #1a1a2e;
    color: #e0e0e0;
  }
  .logo {
    font-size: 56px;
    color: #e0b86a;
    line-height: 1;
    margin-bottom: 8px;
  }
  h1 {
    font-size: 22px;
    font-weight: 700;
    color: #fff;
    margin: 0 0 4px;
  }
  .sub {
    color: #888;
    font-size: 13px;
    margin: 0 0 28px;
  }
  .detecting {
    display: flex;
    align-items: center;
    gap: 10px;
    color: #aaa;
    font-size: 13px;
  }
  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid #334;
    border-top-color: #e0b86a;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .prompt {
    color: #ccc;
    font-size: 13px;
    margin: 0 0 6px;
  }
  .hint {
    color: #666;
    font-size: 11px;
    margin: 0 0 20px;
  }
  .field {
    width: 100%;
    margin-bottom: 10px;
    text-align: left;
  }
  .field label {
    display: block;
    font-size: 11px;
    color: #888;
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .field input {
    width: 100%;
    box-sizing: border-box;
    background: #0f1f3d;
    border: 1px solid #1a4a80;
    border-radius: 6px;
    padding: 9px 12px;
    color: #fff;
    font-size: 14px;
    outline: none;
  }
  .field input:focus { border-color: #e0b86a; }
  .err {
    color: #e06060;
    font-size: 12px;
    margin-bottom: 8px;
    align-self: flex-start;
  }
  .save-btn {
    width: 100%;
    margin-top: 8px;
    padding: 11px;
    background: #e0b86a;
    color: #1a1a2e;
    font-weight: 700;
    font-size: 14px;
    border: none;
    border-radius: 8px;
    cursor: pointer;
  }
  .save-btn:hover:not(:disabled) { background: #f0cc8a; }
  .save-btn:disabled { opacity: 0.5; cursor: wait; }
</style>
