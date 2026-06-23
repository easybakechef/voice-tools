<script lang="ts">
  import { engine } from '$lib/audio/engine.svelte.js';
  import { drawSpectrum, clearCanvas } from '$lib/audio/drawing.js';

  let canvas = $state<HTMLCanvasElement | null>(null);

  $effect(() => {
    const active = engine.activeType;
    if (!canvas) return;
    if (!active) { clearCanvas(canvas); return; }

    // Capture binGroup so the RAF loop always uses the current value.
    // Re-runs when binGroup changes thanks to $effect tracking.
    const bg = engine.binGroup;

    let animId: number;
    const draw = () => {
      animId = requestAnimationFrame(draw);
      drawSpectrum(canvas!, engine.frequencyData, engine.sampleRate, bg);
    };
    animId = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(animId);
  });
</script>

<canvas bind:this={canvas}></canvas>

<!-- Formant legend -->
<div class="legend">
  <span class="legend-item"><span class="swatch" style="background:#e74c6f"></span>F0 fundamental</span>
  <span class="legend-item"><span class="swatch" style="background:#f39c12"></span>F1 first formant</span>
  <span class="legend-item"><span class="swatch" style="background:#5BCEFA"></span>F2 brightness</span>
  <span class="legend-item"><span class="swatch" style="background:#9b59b6"></span>F3 third formant</span>
</div>

<!-- Bin-size slider -->
<div class="bin-row">
  <label for="binSlider">Bin Size</label>
  <input
    id="binSlider"
    type="range"
    min="1" max="48" step="1"
    value={engine.binGroup}
    oninput={(e) => { engine.binGroup = parseInt((e.target as HTMLInputElement).value); }}
  />
  <span class="bin-val">{engine.binGroup === 1 ? '1 bin / bar' : `${engine.binGroup} bins / bar`}</span>
</div>

<style>
  canvas {
    display: block;
    width: 100%;
    height: 160px;
    border-radius: 8px;
    background: #12122a;
  }

  .legend {
    display: flex;
    gap: 1.1rem;
    flex-wrap: wrap;
    margin-top: 0.7rem;
  }
  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.73rem;
    color: var(--muted);
  }
  .swatch {
    width: 11px;
    height: 11px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .bin-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.85rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border);
  }
  .bin-row label {
    font-size: 0.75rem;
    color: var(--muted);
    white-space: nowrap;
  }
  .bin-row input[type='range'] {
    flex: 1;
    accent-color: var(--trans-pink);
    cursor: pointer;
    height: 4px;
  }
  .bin-val {
    min-width: 90px;
    text-align: right;
    font-size: 0.75rem;
    color: var(--trans-pink);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
