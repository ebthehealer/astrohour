<script lang="ts">
  /**
   * Notes.svelte — per-hour note editor.
   *
   * Props:
   *   planetKey  — "YYYY-MM-DD:N" identifying this hour slot
   *   planetName — human-readable label for the header (e.g. "Jupiter Hour")
   *   onClose    — callback to dismiss this overlay
   *
   * Queries and mutates the SQLite notes table via @tauri-apps/plugin-sql.
   * All SQL runs on the JS side — the Rust commands only provide helpers.
   */
  import { onMount } from 'svelte';
  import Database from '@tauri-apps/plugin-sql';
  import type { Note } from './api';

  export let planetKey:  string;
  export let planetName: string;
  export let onClose:    () => void;

  const DB_URL = 'sqlite:astrohour.db';

  let db:      Awaited<ReturnType<typeof Database.load>> | null = null;
  let notes:   Note[] = [];
  let draft:   string = '';
  let editing: Note | null = null;
  let loading  = true;
  let saving   = false;
  let error:   string | null = null;

  // ── DB helpers

  async function openDb() {
    if (!db) db = await Database.load(DB_URL);
    return db;
  }

  async function loadNotes() {
    loading = true;
    error = null;
    try {
      const conn = await openDb();
      notes = await conn.select<Note[]>(
        'SELECT * FROM notes WHERE planet_key = ? ORDER BY created_at DESC',
        [planetKey]
      );
    } catch (e: any) {
      error = e.toString();
    } finally {
      loading = false;
    }
  }

  async function saveNote() {
    if (!draft.trim()) return;
    saving = true;
    error  = null;
    try {
      const conn = await openDb();
      if (editing) {
        await conn.execute(
          'UPDATE notes SET body = ?, updated_at = datetime(\'now\') WHERE id = ?',
          [draft, editing.id]
        );
        editing = null;
      } else {
        await conn.execute(
          'INSERT INTO notes (planet_key, body) VALUES (?, ?)',
          [planetKey, draft]
        );
      }
      draft = '';
      await loadNotes();
    } catch (e: any) {
      error = e.toString();
    } finally {
      saving = false;
    }
  }

  async function deleteNote(id: number) {
    error = null;
    try {
      const conn = await openDb();
      await conn.execute('DELETE FROM notes WHERE id = ?', [id]);
      if (editing?.id === id) { editing = null; draft = ''; }
      await loadNotes();
    } catch (e: any) {
      error = e.toString();
    }
  }

  function startEdit(note: Note) {
    editing = note;
    draft   = note.body;
  }

  function cancelEdit() {
    editing = null;
    draft   = '';
  }

  function formatDate(iso: string) {
    return new Date(iso).toLocaleString([], {
      month: 'short', day: 'numeric',
      hour: '2-digit', minute: '2-digit'
    });
  }

  onMount(loadNotes);
</script>

<!-- Overlay backdrop -->
<div class="backdrop" on:click|self={onClose} role="none">
  <div class="panel">

    <!-- Header -->
    <div class="header">
      <span class="title">{planetName} — Notes</span>
      <button class="close" on:click={onClose} aria-label="Close">×</button>
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <!-- Note list -->
    <div class="list">
      {#if loading}
        <p class="empty">Loading…</p>
      {:else if notes.length === 0}
        <p class="empty">No notes for this hour yet.</p>
      {:else}
        {#each notes as note (note.id)}
          <div class="note" class:editing={editing?.id === note.id}>
            <p class="body">{note.body}</p>
            <div class="meta">
              <span class="ts">{formatDate(note.created_at)}</span>
              <button class="btn-sm" on:click={() => startEdit(note)}>Edit</button>
              <button class="btn-sm danger" on:click={() => deleteNote(note.id)}>Delete</button>
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Editor -->
    <div class="editor">
      <textarea
        bind:value={draft}
        placeholder={editing ? 'Edit note…' : 'Add a note for this hour…'}
        rows="3"
        on:keydown={(e) => { if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) saveNote(); }}
      />
      <div class="editor-actions">
        {#if editing}
          <button class="btn-sm" on:click={cancelEdit}>Cancel</button>
        {/if}
        <button class="btn-primary" on:click={saveNote} disabled={saving || !draft.trim()}>
          {saving ? 'Saving…' : editing ? 'Update' : 'Save'}
        </button>
      </div>
    </div>

  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex;
    align-items: flex-end;
    z-index: 100;
  }
  .panel {
    width: 100%;
    max-height: 80vh;
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
  .close {
    background: none; border: none; color: #888;
    font-size: 1.3rem; cursor: pointer; padding: 0 4px;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 10px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .empty { color: #666; font-size: 0.85rem; text-align: center; margin: 16px 0; }
  .note {
    background: rgba(255,255,255,0.05);
    border-radius: 8px;
    padding: 10px 12px;
    border: 1px solid transparent;
  }
  .note.editing { border-color: #7c6fcd; }
  .body { margin: 0 0 6px; font-size: 0.88rem; color: #d4cff0; white-space: pre-wrap; }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
  }
  .ts { color: #666; flex: 1; }
  .editor {
    padding: 12px 16px;
    border-top: 1px solid rgba(255,255,255,0.08);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  textarea {
    width: 100%;
    background: rgba(255,255,255,0.07);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 8px;
    color: #e0d8f8;
    font-size: 0.88rem;
    padding: 8px 10px;
    resize: none;
    box-sizing: border-box;
    font-family: inherit;
  }
  textarea:focus { outline: none; border-color: #7c6fcd; }
  .editor-actions { display: flex; justify-content: flex-end; gap: 8px; }
  .btn-primary {
    background: #7c6fcd;
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 6px 16px;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .btn-primary:disabled { opacity: 0.5; cursor: default; }
  .btn-sm {
    background: rgba(255,255,255,0.08);
    color: #ccc;
    border: none;
    border-radius: 5px;
    padding: 3px 10px;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .btn-sm.danger { color: #e06c75; }
  .error { color: #e06c75; font-size: 0.8rem; padding: 6px 16px; }
</style>
