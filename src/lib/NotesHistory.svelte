<script lang="ts">
  /**
   * NotesHistory.svelte — browse all notes grouped by date, searchable.
   */
  import { onMount } from 'svelte';
  import Database from '@tauri-apps/plugin-sql';
  import { planetSymbol } from './api';

  export let onClose: () => void;
  export let onOpenNote: (key: string, name: string) => void;

  const DB_URL = 'sqlite:astrohour.db';

  interface NoteRow {
    id: number;
    planet_key: string;   // "YYYY-MM-DD:N"
    planet_name: string | null;  // e.g. "☿ Mercury Hour"
    body: string;
    created_at: string;
    updated_at: string;
  }

  let db: Awaited<ReturnType<typeof Database.load>> | null = null;
  let all:     NoteRow[] = [];
  let query    = '';
  let loading  = true;
  let error: string | null = null;

  async function openDb() {
    if (!db) db = await Database.load(DB_URL);
    return db;
  }

  async function loadAll() {
    loading = true; error = null;
    try {
      const conn = await openDb();
      all = await conn.select<NoteRow[]>(
        `SELECT * FROM notes ORDER BY created_at DESC`
      );
    } catch (e: any) {
      error = e.toString();
    } finally {
      loading = false;
    }
  }

  // Parse planet_key "YYYY-MM-DD:N" → { date, index }
  function parseKey(key: string) {
    const [date, idx] = key.split(':');
    return { date, index: Number(idx) };
  }

  function labelFor(note: NoteRow) {
    if (note.planet_name) return note.planet_name;
    // Fallback for old notes without planet_name stored
    const { index } = parseKey(note.planet_key);
    return `Hour ${index + 1}`;
  }

  function formatDate(dateStr: string) {
    const d = new Date(dateStr + 'T00:00:00');
    return d.toLocaleDateString([], { weekday: 'short', month: 'short', day: 'numeric', year: 'numeric' });
  }

  function formatTs(iso: string) {
    // SQLite stores UTC without 'Z' — append it so JS parses as UTC then converts to local
    const fixed = iso.endsWith('Z') || iso.includes('+') ? iso : iso + 'Z';
    return new Date(fixed).toLocaleString([], {
      month: 'short', day: 'numeric',
      hour: '2-digit', minute: '2-digit'
    });
  }

  // Group filtered notes by date
  $: filtered = query.trim()
    ? all.filter(n => n.body.toLowerCase().includes(query.toLowerCase()))
    : all;

  $: grouped = (() => {
    const map = new Map<string, NoteRow[]>();
    for (const n of filtered) {
      const { date } = parseKey(n.planet_key);
      if (!map.has(date)) map.set(date, []);
      map.get(date)!.push(n);
    }
    return [...map.entries()].sort((a, b) => b[0].localeCompare(a[0]));
  })();

  async function deleteNote(id: number) {
    try {
      const conn = await openDb();
      await conn.execute('DELETE FROM notes WHERE id = ?', [id]);
      await loadAll();
    } catch (e: any) {
      error = e.toString();
    }
  }

  onMount(loadAll);
</script>

<div class="history-panel">
  <div class="hist-header">
    <span class="hist-title">📓 Notes History</span>
    <button class="close-btn" on:click={onClose} aria-label="Close">×</button>
  </div>

  <div class="search-row">
    <input
      class="search-input"
      type="text"
      bind:value={query}
      placeholder="Search notes…"
    />
  </div>

  {#if error}
    <div class="error-msg">{error}</div>
  {/if}

  <div class="hist-body">
    {#if loading}
      <p class="empty">Loading…</p>
    {:else if filtered.length === 0}
      <p class="empty">{query ? 'No notes match your search.' : 'No notes yet. Click ✏ on any hour to add one.'}</p>
    {:else}
      {#each grouped as [date, notes]}
        <div class="date-group">
          <div class="date-label">{formatDate(date)}</div>
          {#each notes as note (note.id)}
            {@const label = labelFor(note)}
            <div class="hist-note">
              <div class="hist-note-meta">
                <span class="hist-hour"
                  role="button"
                  tabindex="0"
                  on:click={() => onOpenNote(note.planet_key, label)}
                  on:keydown={(e) => e.key === 'Enter' && onOpenNote(note.planet_key, label)}
                >{label}</span>
                <span class="hist-ts">{formatTs(note.created_at)}</span>
                <button class="del-btn" on:click={() => deleteNote(note.id)} title="Delete">×</button>
              </div>
              <p class="hist-body-text">{note.body}</p>
            </div>
          {/each}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .history-panel {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-base, #000);
    overflow: hidden;
  }
  .hist-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px 10px;
    border-bottom: 1px solid var(--bg-elevated);
    flex-shrink: 0;
  }
  .hist-title { font-weight: 700; font-size: 14px; color: var(--text-primary); }
  .close-btn {
    background: none; border: none;
    color: var(--text-secondary);
    font-size: 20px; cursor: pointer; padding: 0 4px; line-height: 1;
  }
  .close-btn:hover { color: var(--text-primary); }

  .search-row {
    padding: 10px 16px 8px;
    flex-shrink: 0;
    border-bottom: 1px solid var(--bg-elevated);
  }
  .search-input {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-surface);
    border: 1px solid var(--bg-elevated);
    border-radius: 6px;
    padding: 7px 10px;
    color: var(--text-primary);
    font-size: 13px;
    outline: none;
    font-family: inherit;
  }
  .search-input:focus { border-color: var(--accent); }
  .search-input::placeholder { color: var(--text-muted); }

  .hist-body {
    flex: 1;
    overflow-y: auto;
    padding: 0 16px 16px;
  }
  .empty {
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
    margin: 24px 0;
  }
  .date-group { margin-top: 16px; }
  .date-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
    margin-bottom: 8px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--bg-elevated);
  }
  .hist-note {
    background: var(--bg-surface);
    border-radius: 8px;
    padding: 9px 11px;
    margin-bottom: 6px;
    border: 1px solid var(--bg-elevated);
  }
  .hist-note-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 5px;
  }
  .hist-hour {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .hist-hour:hover { color: var(--text-primary); }
  .hist-ts {
    font-size: 11px;
    color: var(--text-muted);
    flex: 1;
  }
  .del-btn {
    background: none; border: none;
    color: var(--text-muted);
    font-size: 15px; cursor: pointer; padding: 0 2px; line-height: 1;
    opacity: 0.5;
  }
  .del-btn:hover { color: var(--danger); opacity: 1; }
  .hist-body-text {
    margin: 0;
    font-size: 13px;
    color: var(--text-primary);
    white-space: pre-wrap;
    line-height: 1.5;
  }
  .error-msg {
    color: var(--danger);
    font-size: 12px;
    padding: 6px 16px;
  }
</style>
