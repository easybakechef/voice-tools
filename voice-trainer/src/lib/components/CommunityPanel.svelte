<script lang="ts">
  import { onMount } from 'svelte';
  import { listPublicRecordings, getAudioUrl, type PublicRecording } from '$lib/data/recordings.js';
  import { engine } from '$lib/audio/engine.svelte.js';
  import PublicRecordingCard from './PublicRecordingCard.svelte';

  let recordings = $state<PublicRecording[]>([]);
  let loading    = $state(true);
  let error      = $state('');
  let playingId  = $state<string | null>(null);

  async function load() {
    loading = true;
    error = '';
    try {
      recordings = await listPublicRecordings();
    } catch (e) {
      error = `Could not load the community feed — is Supabase running?`;
      console.error(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

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
      <div class="card-label">Community Feedback</div>
      <p class="intro">Listen to recordings others have shared and leave them feedback.</p>
    </div>
    <button class="refresh" onclick={load} disabled={loading}>↻ Refresh</button>
  </div>

  {#if loading}
    <p class="muted">Loading shared recordings…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if !recordings.length}
    <p class="muted">
      No shared recordings yet. When someone marks a recording public on their
      Library → Feedback tab, it shows up here.
    </p>
  {:else}
    <div class="feed">
      {#each recordings as rec (rec.id)}
        <PublicRecordingCard recording={rec} isPlaying={playingId === rec.id} onPlay={play} />
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

  .feed { display: flex; flex-direction: column; gap: 1rem; margin-top: 1rem; }
  .muted { color: var(--muted); font-size: 0.85rem; line-height: 1.6; margin-top: 1rem; }
  .error { color: #e74c3c; font-size: 0.85rem; margin-top: 1rem; }
</style>
