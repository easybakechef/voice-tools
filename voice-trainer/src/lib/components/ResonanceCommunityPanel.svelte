<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listPublicResonancePairs, type ResonancePair, type ResonanceSample } from '$lib/data/resonance.js';
  import { getSampleUrl } from '$lib/data/dataset.js';
  import ResonancePairCard from './ResonancePairCard.svelte';

  let pairs   = $state<ResonancePair[]>([]);
  let loading = $state(true);
  let error   = $state('');
  let playingSampleId = $state<string | null>(null);

  let audio: HTMLAudioElement | null = null;

  async function load() {
    loading = true; error = '';
    try { pairs = await listPublicResonancePairs(); }
    catch (e) { error = `Could not load the resonance community — is Supabase running?`; console.error(e); }
    finally { loading = false; }
  }
  onMount(load);

  function stopAudio() {
    if (audio) { audio.pause(); audio = null; }
    playingSampleId = null;
  }

  async function play(s: ResonanceSample) {
    if (playingSampleId === s.id) { stopAudio(); return; }
    stopAudio();
    const url = await getSampleUrl(s.storagePath);
    audio = new Audio(url);
    audio.onended = () => { playingSampleId = null; audio = null; };
    playingSampleId = s.id;
    await audio.play();
  }

  onDestroy(stopAudio);
</script>

<div class="card">
  <div class="card-head">
    <div>
      <div class="card-label">Resonance Community</div>
      <p class="intro">
        Listen to both takes of a phrase and guess which sounds brighter. After you choose,
        we reveal how the speaker labelled them and how everyone else voted.
      </p>
    </div>
    <button class="refresh" onclick={load} disabled={loading}>↻ Refresh</button>
  </div>

  {#if loading}
    <p class="muted">Loading shared pairs…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if !pairs.length}
    <p class="muted">
      No shared pairs yet. Record a pair under Resonance Recording, then mark it public
      in “My recordings” and it will show up here for others to judge.
    </p>
  {:else}
    <div class="feed">
      {#each pairs as pair (pair.id)}
        <ResonancePairCard {pair} {playingSampleId} onPlay={play} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .card-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
  .intro { font-size: 0.82rem; color: var(--muted); margin: 0.25rem 0 0; line-height: 1.5; }
  .refresh { flex-shrink: 0; padding: 0.4rem 0.9rem; border-radius: 20px; border: 1px solid var(--border); background: transparent; color: var(--muted); font-size: 0.8rem; font-weight: 600; cursor: pointer; }
  .refresh:hover:not(:disabled) { color: var(--text); }
  .refresh:disabled { opacity: 0.5; cursor: default; }
  .feed { display: flex; flex-direction: column; gap: 1rem; margin-top: 1rem; }
  .muted { color: var(--muted); font-size: 0.85rem; line-height: 1.6; margin-top: 1rem; }
  .error { color: #e74c3c; font-size: 0.85rem; margin-top: 1rem; }
</style>
