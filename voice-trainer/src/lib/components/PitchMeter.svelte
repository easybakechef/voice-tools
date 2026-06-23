<script lang="ts">
  import { engine } from '$lib/audio/engine.svelte.js';
  import { hzToPct } from '$lib/audio/drawing.js';
  import { TARGET_LO, TARGET_HI } from '$lib/audio/constants.js';

  const targetLeft  = hzToPct(TARGET_LO);
  const targetWidth = hzToPct(TARGET_HI) - targetLeft;
  const SCALE_LABELS = [80, 120, 165, 200, 255, 350];
</script>

<div class="card">
  <div class="card-label">Pitch</div>
  <div class="pitch-row">
    <div class="pitch-display">
      <div class="pitch-hz">
        {engine.smoothPitch !== null ? Math.round(engine.smoothPitch) : '—'}
      </div>
      <div class="pitch-label">Hz (Pitch)</div>
    </div>

    <div class="pitch-bar-wrap">
      <div class="pitch-track">
        <div
          class="target-zone"
          style="left: {targetLeft}%; width: {targetWidth}%"
        ></div>
        {#if engine.smoothPitch !== null}
          <div
            class="pitch-needle"
            style="left: {hzToPct(engine.smoothPitch)}%;
                   background: {engine.pitchInTarget ? 'var(--trans-pink)' : '#f39c12'}"
          ></div>
        {/if}
      </div>

      <div class="pitch-scale">
        {#each SCALE_LABELS as hz}
          <span>{hz}</span>
        {/each}
      </div>

      <div
        class="pitch-hint"
        style="color: {engine.pitchInTarget ? 'var(--trans-pink)' : '#f39c12'}"
      >
        {engine.pitchHint}
      </div>
    </div>
  </div>
</div>

<style>
  .pitch-row {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    flex-wrap: wrap;
  }
  .pitch-display {
    min-width: 110px;
    text-align: center;
  }
  .pitch-hz {
    font-size: 2.8rem;
    font-weight: 700;
    line-height: 1;
    color: var(--trans-pink);
    font-variant-numeric: tabular-nums;
  }
  .pitch-label {
    font-size: 0.7rem;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .pitch-bar-wrap {
    flex: 1;
    min-width: 200px;
  }
  .pitch-track {
    position: relative;
    height: 36px;
    background: #12122a;
    border-radius: 18px;
    overflow: hidden;
  }
  .target-zone {
    position: absolute;
    top: 0; bottom: 0;
    background: rgba(91,206,250,0.13);
    border-left: 2px solid var(--trans-blue);
    border-right: 2px solid var(--trans-pink);
  }
  .pitch-needle {
    position: absolute;
    top: 4px; bottom: 4px;
    width: 4px;
    border-radius: 2px;
    background: var(--trans-pink);
    transition: left 0.08s;
    transform: translateX(-50%);
  }
  .pitch-scale {
    display: flex;
    justify-content: space-between;
    margin-top: 4px;
    font-size: 0.65rem;
    color: var(--muted);
  }
  .pitch-hint {
    margin-top: 0.4rem;
    font-size: 0.8rem;
    min-height: 1.2em;
  }
</style>
