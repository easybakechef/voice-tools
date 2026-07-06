<script lang="ts">
  import { engine } from '$lib/audio/engine.svelte.js';

  const res = $derived(engine.resonance);          // 0–100 or null
  const pos = $derived(res ?? 50);                 // needle position
</script>

<div class="card">
  <div class="head">
    <div class="card-label">Resonance</div>
    <div class="readout">
      {#if res != null}
        <span class="pct">{res}%</span>
        {#if engine.resonanceVowel}<span class="vowel">≈ {engine.resonanceVowel}</span>{/if}
      {:else}
        <span class="idle">— speak to measure</span>
      {/if}
    </div>
  </div>

  <div class="track" class:live={res != null}>
    <div class="mid"></div>
    <div class="needle" style="left:{pos}%"></div>
  </div>
  <div class="scale">
    <span>deeper</span>
    <span>average</span>
    <span>brighter</span>
  </div>

  <p class="note">
    Higher = brighter resonance (formants raised relative to the average for the vowel you're
    saying). This is a live approximation — for an accurate score, resonance is best measured on a
    recording of known text.
  </p>
</div>

<style>
  .head { display: flex; align-items: baseline; justify-content: space-between; gap: 1rem; }
  .readout { display: inline-flex; align-items: baseline; gap: 0.5rem; }
  .pct { font-size: 1.5rem; font-weight: 800; color: var(--text); font-variant-numeric: tabular-nums; }
  .vowel { font-size: 0.78rem; color: var(--muted); }
  .idle { font-size: 0.82rem; color: var(--muted); }

  .track {
    position: relative;
    height: 20px;
    border-radius: 10px;
    margin: 0.85rem 0 0.4rem;
    background: linear-gradient(90deg, #7d6cff 0%, #5BCEFA 50%, #F5A9B8 100%);
    opacity: 0.5;
    transition: opacity 0.2s;
  }
  .track.live { opacity: 1; }
  .mid { position: absolute; left: 50%; top: -3px; bottom: -3px; width: 1px; background: rgba(255,255,255,0.35); }
  .needle {
    position: absolute; top: -4px; width: 4px; height: 28px;
    background: #fff; border-radius: 2px; transform: translateX(-50%);
    box-shadow: 0 0 6px rgba(0,0,0,0.5);
    transition: left 0.08s linear;
  }

  .scale { display: flex; justify-content: space-between; font-size: 0.68rem; color: var(--muted); }
  .note { font-size: 0.74rem; color: var(--muted); line-height: 1.5; margin-top: 0.75rem; }
</style>
