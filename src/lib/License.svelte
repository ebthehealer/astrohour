<script lang="ts">
  /**
   * License.svelte — license key entry and status display.
   *
   * Flow:
   *   1. On mount, reads key_hash + activated_at from SQLite license table.
   *   2. If a hash exists, calls restore_license to re-activate in Rust state.
   *   3. User can enter a new key — activateLicense validates + signs in Rust,
   *      then we persist raw key to stronghold and hash to SQLite.
   *
   * Stronghold storage (JS side):
   *   Vault:  'astrohour'
   *   Record: 'license_key'
   *   Password: device_secret from prefs store
   */
  import { onMount, createEventDispatcher } from 'svelte';
  import Database from '@tauri-apps/plugin-sql';
  import {
    activateLicense, deactivateLicense,
    getLicenseStatus, restoreLicense,
    type LicenseStatus
  } from './api';

  export let onClose: () => void;
  export let onActivated: () => void;

  const DB_URL = 'sqlite:astrohour.db';

  let status: LicenseStatus = { status: 'none' };
  let keyInput   = '';
  let submitting = false;
  let error: string | null = null;
  let success: string | null = null;

  // ── Helpers

  async function openDb() {
    return Database.load(DB_URL);
  }

  async function loadStatus() {
    try {
      const db = await openDb();
      const rows = await db.select<{ key_hash: string | null; activated_at: string | null }[]>(
        'SELECT key_hash, activated_at FROM license WHERE id = 1'
      );
      const row = rows[0];
      if (row?.key_hash) {
        await restoreLicense(row.key_hash, row.activated_at ?? null);
      }
      status = await getLicenseStatus();
    } catch (e: any) {
      error = e.toString();
    }
  }

  async function handleActivate() {
    const key = keyInput.trim().toUpperCase();
    if (!key) return;
    submitting = true;
    error   = null;
    success = null;
    try {
      // Validate + record in Rust state — returns key_hash on success
      const keyHash = await activateLicense(key);

      // Persist hash to SQLite (so we can restore on next launch)
      const now = new Date().toISOString();
      const db  = await openDb();
      await db.execute(
        'UPDATE license SET key_hash = ?, activated_at = ? WHERE id = 1',
        [keyHash, now]
      );

      // TODO: persist raw key to stronghold for offline re-verification
      // Stronghold JS API requires a vault password (device_secret);
      // wired here once device_secret generation is added to prefs flow.

      status  = await getLicenseStatus();
      keyInput = '';
      success  = 'Premium unlocked! Thank you.';
      onActivated();
    } catch (e: any) {
      error = e.toString();
    } finally {
      submitting = false;
    }
  }

  async function handleDeactivate() {
    await deactivateLicense();
    const db = await openDb();
    await db.execute('UPDATE license SET key_hash = NULL, activated_at = NULL WHERE id = 1');
    status = { status: 'none' };
  }

  onMount(loadStatus);
</script>

<div class="backdrop" on:click|self={onClose} role="none">
  <div class="panel">

    <div class="header">
      <span class="title">AstroHour Premium</span>
      <button class="close" on:click={onClose} aria-label="Close">×</button>
    </div>

    <div class="body">

      {#if status.status === 'active'}
        <!-- Active -->
        <div class="badge active">✓ Premium Active</div>
        <p class="detail">
          Activated {new Date(status.activated_at).toLocaleDateString()}<br>
          Key: <code>{status.key_hash.slice(0,12)}…</code>
        </p>
        <button class="btn-sm danger" on:click={handleDeactivate}>Deactivate</button>

      {:else}
        <!-- Not active -->
        <div class="features">
          <p class="lead">Unlock premium for a one-time payment of <strong>$10</strong>:</p>
          <ul>
            <li>Full 24-hour schedule view</li>
            <li>Per-hour notes with history</li>
            <li>Time travel — browse past &amp; future dates</li>
            <li>Hour-change notifications</li>
            <li>All display modes (Icon, Short, Full)</li>
          </ul>
          <a
            href="https://astrohour.app/buy"
            target="_blank"
            rel="noreferrer"
            class="btn-buy"
          >
            Buy — $10 one-time
          </a>
        </div>

        <div class="divider"></div>

        <!-- Key entry -->
        <p class="label">Already have a key?</p>
        {#if error}
          <div class="error">{error}</div>
        {/if}
        {#if success}
          <div class="success">{success}</div>
        {/if}
        <input
          type="text"
          bind:value={keyInput}
          placeholder="ASTROHOUR-XXXX-XXXX-XXXX-XXXX"
          spellcheck="false"
          autocomplete="off"
          on:keydown={(e) => { if (e.key === 'Enter') handleActivate(); }}
        />
        <button
          class="btn-primary"
          on:click={handleActivate}
          disabled={submitting || !keyInput.trim()}
        >
          {submitting ? 'Validating…' : 'Activate'}
        </button>
      {/if}

    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    display: flex;
    align-items: flex-end;
    z-index: 100;
  }
  .panel {
    width: 100%;
    background: #1a1a2e;
    border-radius: 14px 14px 0 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px 10px;
    border-bottom: 1px solid rgba(255,255,255,0.08);
  }
  .title { font-weight: 600; font-size: 0.95rem; color: #e0d8f8; }
  .close { background: none; border: none; color: #888; font-size: 1.3rem; cursor: pointer; }
  .body { padding: 16px; display: flex; flex-direction: column; gap: 12px; }
  .badge {
    display: inline-block;
    padding: 4px 12px;
    border-radius: 20px;
    font-size: 0.85rem;
    font-weight: 600;
  }
  .badge.active { background: rgba(80,200,120,0.15); color: #50c878; }
  .detail { font-size: 0.82rem; color: #888; margin: 0; }
  .detail code { color: #b8b0e0; }
  .features { display: flex; flex-direction: column; gap: 8px; }
  .lead { margin: 0; font-size: 0.88rem; color: #c8c0e8; }
  ul { margin: 0; padding-left: 18px; color: #a0a0c0; font-size: 0.84rem; }
  ul li { margin: 3px 0; }
  .btn-buy {
    display: block;
    text-align: center;
    background: linear-gradient(135deg, #7c6fcd, #a08ee0);
    color: #fff;
    text-decoration: none;
    padding: 10px;
    border-radius: 8px;
    font-weight: 600;
    font-size: 0.9rem;
    margin-top: 4px;
  }
  .divider { height: 1px; background: rgba(255,255,255,0.07); }
  .label { margin: 0; font-size: 0.82rem; color: #888; }
  input {
    width: 100%;
    background: rgba(255,255,255,0.07);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 8px;
    color: #e0d8f8;
    font-size: 0.85rem;
    padding: 8px 10px;
    box-sizing: border-box;
    letter-spacing: 0.04em;
  }
  input:focus { outline: none; border-color: #7c6fcd; }
  .btn-primary {
    background: #7c6fcd;
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 9px;
    font-size: 0.88rem;
    font-weight: 600;
    cursor: pointer;
    width: 100%;
  }
  .btn-primary:disabled { opacity: 0.5; cursor: default; }
  .btn-sm {
    background: rgba(255,255,255,0.07);
    color: #ccc;
    border: none;
    border-radius: 6px;
    padding: 5px 14px;
    font-size: 0.8rem;
    cursor: pointer;
    width: fit-content;
  }
  .btn-sm.danger { color: #e06c75; }
  .error   { color: #e06c75; font-size: 0.82rem; }
  .success { color: #50c878; font-size: 0.82rem; }
</style>
