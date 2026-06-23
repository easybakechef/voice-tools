<script lang="ts">
  import { onMount } from 'svelte';
  import { listPublicRecordings, listMyPublicRecordings, getAudioUrl, type PublicRecording } from '$lib/data/recordings.js';
  import { engine } from '$lib/audio/engine.svelte.js';
  import PublicRecordingCard from './PublicRecordingCard.svelte';

  type View = 'others' | 'mine';
  let view = $state<View>('others');

  let recordings = $state<PublicRecording[]>([]);
  let loading    = $state(true);
  let error      = $state('');
  let playingId  = $state<string | null>(null);

  async function load() {
    loading = true;
    error = '';
    try {
      recordings = view === 'mine' ? await listMyPublicRecordings() : await listPublicRecordings();
    } catch (e) {
      error = `Could not load recordings — is Supabase running?`;
      console.error(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function switchView(v: View) {
    if (v === view) return;
    engine.stopAll();
    playingId = null;
    view = v;
    load();
  }

  // Reset our local "playing" marker whenever the shared engine stops.
  $effect(() => {
    if (engine.activeType === null) playingId = null;
  });

  async function play(rec: PublicRecording) {
    if (playingId === rec.id) {
      engine.stopAll();
      playingId = null;
      return;
    }
    playingId = rec.id;
    const url = await getAudioUrl(rec.id);
    await engine.loadUrl(url, rec.name);
  }
</script>

<div class="card">
  <div class="card-head">
    <div>
      <div class="card-label">{view === 'mine' ? 'My Shared Recordings' : 'Community Feedback'}</div>
      <p class="intro">
        {view === 'mine'
          ? 'Your public recordings and the feedback others have left on them.'
          : 'Listen to recordings others have shared and leave them feedback.'}
      </p>
    </div>
    <button class="refresh" onclick={load} disabled={loading}>↻ Refresh</button>
  </div>

  <div class="seg">
    <button class:active={view === 'others'} onclick={() => switchView('others')}>Community</button>
    <button class:active={view === 'mine'} onclick={() => switchView('mine')}>My shared</button>
  </div>

  {#if loading}
    <p class="muted">Loading recordings…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if !recordings.length}
    {#if view === 'mine'}
      <p class="muted">
        You haven't shared any recordings yet. Mark a recording public on the
        Recording → Library → Feedback tab and it will appear here.
      </p>
    {:else}
      <p class="muted">
        No shared recordings yet. When someone marks a recording public, it shows up here.
      </p>
    {/if}
  {:else}
    <div class="feed">
      {#each recordings as rec (rec.id)}
        <PublicRecordingCard recording={rec} isPlaying={playingId === rec.id} onPlay={play} mine={view === 'mine'} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .card-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
  .intro { font-size: 0.82rem; color: var(--muted); margin: 0.25rem 0 0; }
  .refresh {
    flex-shrink: 0;
    padding: 0.4rem 0.9rem;
    border-radius: 20px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .refresh:hover:not(:disabled) { color: var(--text); }
  .refresh:disabled { opacity: 0.5; cursor: default; }

  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 20px; overflow: hidden; margin-top: 0.85rem; }
  .seg button { background: transparent; border: none; color: var(--muted); font-size: 0.8rem; font-weight: 600; padding: 0.4rem 1.1rem; cursor: pointer; }
  .seg button.active { background: var(--trans-pink); color: #0d0d24; }

  .feed { display: flex; flex-direction: column; gap: 1rem; margin-top: 1rem; }
  .muted { color: var(--muted); font-size: 0.85rem; line-height: 1.6; margin-top: 1rem; }
  .error { color: #e74c3c; font-size: 0.85rem; margin-top: 1rem; }
</style>
