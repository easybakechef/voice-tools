<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listPublicResonancePairs, listMyPublicResonancePairs, type ResonancePair, type ResonanceSample } from '$lib/data/resonance.js';
  import { getSampleUrl } from '$lib/data/dataset.js';
  import ResonancePairCard from './ResonancePairCard.svelte';

  type View = 'others' | 'mine';
  let view = $state<View>('others');

  let pairs   = $state<ResonancePair[]>([]);
  let loading = $state(true);
  let error   = $state('');
  let playingSampleId = $state<string | null>(null);

  let audio: HTMLAudioElement | null = null;

  async function load() {
    loading = true; error = '';
    try { pairs = view === 'mine' ? await listMyPublicResonancePairs() : await listPublicResonancePairs(); }
    catch (e) { error = `Could not load the resonance community — is Supabase running?`; console.error(e); }
    finally { loading = false; }
  }
  onMount(load);

  function switchView(v: View) {
    if (v === view) return;
    stopAudio();
    view = v;
    load();
  }

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
      <div class="card-label">{view === 'mine' ? 'My Shared Pairs' : 'Resonance Community'}</div>
      <p class="intro">
        {#if view === 'mine'}
          Your published pairs, with how people voted and the comments they left.
        {:else}
          Listen to both takes of a phrase and guess which sounds brighter. After you choose,
          we reveal how the speaker labelled them and how everyone else voted.
        {/if}
      </p>
    </div>
    <button class="refresh" onclick={load} disabled={loading}>↻ Refresh</button>
  </div>

  <div class="seg">
    <button class:active={view === 'others'} onclick={() => switchView('others')}>Community</button>
    <button class:active={view === 'mine'} onclick={() => switchView('mine')}>My shared</button>
  </div>

  {#if loading}
    <p class="muted">Loading pairs…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if !pairs.length}
    {#if view === 'mine'}
      <p class="muted">
        You haven't published any pairs yet. Record one under Resonance Recording, then
        “Make public” in My recordings — it'll appear here with its results and comments.
      </p>
    {:else}
      <p class="muted">
        No shared pairs yet. Record a pair under Resonance Recording, then mark it public
        in “My recordings” and it will show up here for others to judge.
      </p>
    {/if}
  {:else}
    <div class="feed">
      {#each pairs as pair (pair.id)}
        <ResonancePairCard {pair} {playingSampleId} onPlay={play} owner={view === 'mine'} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .card-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
  .intro { font-size: 0.82rem; color: var(--muted); margin: 0.25rem 0 0; line-height: 1.5; }
  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 20px; overflow: hidden; margin-top: 0.85rem; }
  .seg button { background: transparent; border: none; color: var(--muted); font-size: 0.8rem; font-weight: 600; padding: 0.4rem 1.1rem; cursor: pointer; }
  .seg button.active { background: var(--trans-pink); color: #0d0d24; }
  .refresh { flex-shrink: 0; padding: 0.4rem 0.9rem; border-radius: 20px; border: 1px solid var(--border); background: transparent; color: var(--muted); font-size: 0.8rem; font-weight: 600; cursor: pointer; }
  .refresh:hover:not(:disabled) { color: var(--text); }
  .refresh:disabled { opacity: 0.5; cursor: default; }
  .feed { display: flex; flex-direction: column; gap: 1rem; margin-top: 1rem; }
  .muted { color: var(--muted); font-size: 0.85rem; line-height: 1.6; margin-top: 1rem; }
  .error { color: #e74c3c; font-size: 0.85rem; margin-top: 1rem; }
</style>
