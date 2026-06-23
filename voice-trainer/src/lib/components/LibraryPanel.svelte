<script lang="ts">
  import { onMount } from 'svelte';
  import { listRecordings, deleteRecording, getAudioUrl, type SavedRecording, type Visibility } from '$lib/data/recordings.js';
  import { engine } from '$lib/audio/engine.svelte.js';
  import StoredSnapshotCard from './StoredSnapshotCard.svelte';
  import RecordingFeedback  from './RecordingFeedback.svelte';
  import AnalysisView       from './AnalysisView.svelte';

  let recordings  = $state<SavedRecording[]>([]);
  let loading     = $state(true);
  let error       = $state('');
  let selected    = $state<SavedRecording | null>(null);
  let detailTab   = $state<'snapshot' | 'playback' | 'feedback'>('snapshot');
  let isPlaying   = $state(false);

  // Reflect a visibility change from the Feedback tab back into local state so
  // the toggle, the sidebar badge, and a later re-select all stay in sync.
  function applyVisibility(v: Visibility) {
    if (!selected) return;
    selected = { ...selected, visibility: v };
    recordings = recordings.map(r => r.id === selected!.id ? { ...r, visibility: v } : r);
  }

  onMount(async () => {
    try {
      recordings = await listRecordings();
    } catch (e) {
      error = `Could not load recordings — is Supabase running? (npx supabase start)`;
      console.error(e);
    } finally {
      loading = false;
    }
  });

  // Clear playing state when engine stops
  $effect(() => {
    if (engine.activeType === null) isPlaying = false;
  });

  function select(rec: SavedRecording) {
    if (selected?.id !== rec.id) {
      // Stop any in-progress playback when switching records
      if (isPlaying) { engine.stopAll(); isPlaying = false; }
      selected  = rec;
      detailTab = 'snapshot';
    }
  }

  async function togglePlay() {
    if (!selected) return;
    if (isPlaying) {
      engine.stopAll();
      isPlaying = false;
    } else {
      isPlaying = true;
      const url = await getAudioUrl(selected.id);
      await engine.loadUrl(url, selected.name);
    }
  }

  async function remove(rec: SavedRecording, e: MouseEvent) {
    e.stopPropagation();
    if (isPlaying && selected?.id === rec.id) engine.stopAll();
    await deleteRecording(rec.id);
    recordings = recordings.filter(r => r.id !== rec.id);
    if (selected?.id === rec.id) selected = null;
  }

  function fmtDur(s: number) {
    return `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
  }

  function fmtDate(ts: number) {
    return new Date(ts).toLocaleString(undefined, {
      month: 'short', day: 'numeric', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }
</script>

<div class="library-layout">
  <!-- ── Left sidebar ─────────────────────────────── -->
  <aside class="sidebar">
    <div class="sidebar-head">Recordings</div>

    {#if loading}
      <p class="empty">Loading…</p>
    {:else if error}
      <p class="empty error">{error}</p>
    {:else if !recordings.length}
      <p class="empty">No saved recordings yet.<br />Record and save a sample on the Live tab.</p>
    {:else}
      <ul class="rec-list">
        {#each recordings as rec (rec.id)}
          <li
            class="rec-item"
            class:active={selected?.id === rec.id}
            onclick={() => select(rec)}
            role="button"
            tabindex="0"
            onkeydown={e => e.key === 'Enter' && select(rec)}
          >
            <div class="rec-item-name">
              {rec.name}
              {#if rec.visibility === 'public'}<span class="pub-dot" title="Shared for feedback">🌐</span>{/if}
            </div>
            <div class="rec-item-meta">
              {fmtDur(rec.duration)} · {Math.round(rec.medianPitch)} Hz
            </div>
            <div class="rec-item-date">{fmtDate(rec.date)}</div>
            <button class="del-btn" onclick={e => remove(rec, e)} title="Delete">✕</button>
          </li>
        {/each}
      </ul>
    {/if}
  </aside>

  <!-- ── Right detail pane ─────────────────────────── -->
  <section class="detail">
    {#if !selected}
      <div class="empty-detail">
        <div class="empty-detail-icon">🎙</div>
        <p>Select a recording to view its snapshot</p>
      </div>
    {:else}
      <div class="detail-header">
        <h2 class="detail-title">{selected.name}</h2>
        <span class="detail-meta">{fmtDate(selected.date)} · {fmtDur(selected.duration)}</span>
      </div>

      <div class="detail-tabs">
        <button
          class="dtab" class:active={detailTab === 'snapshot'}
          onclick={() => detailTab = 'snapshot'}
        >Snapshot</button>
        <button
          class="dtab" class:active={detailTab === 'playback'}
          onclick={() => detailTab = 'playback'}
        >Playback</button>
        <button
          class="dtab" class:active={detailTab === 'feedback'}
          onclick={() => detailTab = 'feedback'}
        >Feedback</button>
      </div>

      {#if detailTab === 'snapshot'}
        <StoredSnapshotCard recording={selected} />

      {:else if detailTab === 'feedback'}
        <RecordingFeedback recording={selected} onVisibilityChange={applyVisibility} />

      {:else}
        <div class="playback-controls">
          <button class="play-btn" class:stop={isPlaying} onclick={togglePlay}>
            {isPlaying ? '■ Stop' : '▶ Play Recording'}
          </button>
          {#if isPlaying}
            <div class="progress-wrap">
              <div class="progress-bar">
                <div class="progress-fill" style="width:{engine.playbackProgress * 100}%"></div>
              </div>
              <span class="progress-time">{engine.playbackTime}</span>
            </div>
          {/if}
        </div>
        <AnalysisView />
      {/if}
    {/if}
  </section>
</div>

<style>
  .library-layout {
    display: flex;
    gap: 1.25rem;
    align-items: flex-start;
    min-height: 500px;
  }

  /* ── Sidebar ── */
  .sidebar {
    width: 260px;
    flex-shrink: 0;
    background: #12122a;
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
  }
  .sidebar-head {
    padding: 0.75rem 1rem;
    font-size: 0.72rem;
    font-weight: 700;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    border-bottom: 1px solid var(--border);
  }
  .rec-list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 70vh;
    overflow-y: auto;
  }
  .rec-item {
    position: relative;
    padding: 0.75rem 1rem;
    cursor: pointer;
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
  }
  .rec-item:hover { background: #1a1a3a; }
  .rec-item.active { background: #1e1e45; border-left: 3px solid var(--trans-pink); }
  .rec-item-name {
    font-weight: 600;
    font-size: 0.875rem;
    color: var(--text);
    padding-right: 1.5rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pub-dot { font-size: 0.7rem; margin-left: 0.25rem; }
  .rec-item-meta {
    font-size: 0.72rem;
    color: var(--trans-blue);
    margin-top: 0.15rem;
  }
  .rec-item-date {
    font-size: 0.68rem;
    color: var(--muted);
    margin-top: 0.1rem;
  }
  .del-btn {
    position: absolute;
    top: 0.65rem;
    right: 0.65rem;
    background: transparent;
    border: none;
    color: var(--muted);
    cursor: pointer;
    font-size: 0.75rem;
    padding: 0.2rem 0.3rem;
    border-radius: 4px;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s;
  }
  .rec-item:hover .del-btn { opacity: 1; }
  .del-btn:hover { color: #e74c3c; }

  .empty {
    padding: 1.5rem 1rem;
    color: var(--muted);
    font-size: 0.8rem;
    text-align: center;
    line-height: 1.6;
  }
  .empty.error { color: #e74c3c; }

  /* ── Detail pane ── */
  .detail { flex: 1; min-width: 0; }

  .empty-detail {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 300px;
    color: var(--muted);
    gap: 0.75rem;
  }
  .empty-detail-icon { font-size: 2.5rem; }
  .empty-detail p { font-size: 0.875rem; }

  .detail-header {
    margin-bottom: 1rem;
  }
  .detail-title {
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--text);
    margin: 0 0 0.2rem;
  }
  .detail-meta {
    font-size: 0.75rem;
    color: var(--muted);
  }

  .detail-tabs {
    display: flex;
    gap: 0.25rem;
    margin-bottom: 1.25rem;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0;
  }
  .dtab {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 0.5rem 1rem;
    color: var(--muted);
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }
  .dtab.active { color: var(--trans-pink); border-bottom-color: var(--trans-pink); }
  .dtab:hover:not(.active) { color: var(--text); }

  .playback-controls {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }
  .play-btn {
    background: var(--trans-blue);
    color: #0d0d24;
    border: none;
    border-radius: 8px;
    padding: 0.5rem 1.25rem;
    font-weight: 700;
    font-size: 0.875rem;
    cursor: pointer;
    transition: background 0.15s;
    flex-shrink: 0;
  }
  .play-btn.stop { background: var(--trans-pink); }

  .progress-wrap {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1;
    min-width: 120px;
  }
  .progress-bar {
    flex: 1;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: var(--trans-pink);
    border-radius: 2px;
    transition: width 0.1s linear;
  }
  .progress-time {
    font-size: 0.75rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
