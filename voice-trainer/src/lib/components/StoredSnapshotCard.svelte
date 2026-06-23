<script lang="ts">
  import { renderPitchTimeline, renderFormantPlotFromPoints, resizeDPR } from '$lib/audio/drawing.js';
  import { TARGET_LO } from '$lib/audio/constants.js';
  import type { SavedRecording } from '$lib/data/recordings.js';

  let { recording }: { recording: SavedRecording } = $props();

  let pitchCanvas = $state<HTMLCanvasElement | null>(null);
  let specCanvas  = $state<HTMLCanvasElement | null>(null);

  $effect(() => {
    if (!pitchCanvas || !specCanvas) return;
    // Access recording fields to track them as reactive deps
    const pl = recording.pitchLog;
    const fd = recording.formantData;
    resizeDPR(pitchCanvas);
    resizeDPR(specCanvas);
    if (pl.length)  renderPitchTimeline(pitchCanvas, pl);
    if (fd.length)  renderFormantPlotFromPoints(specCanvas, fd);
  });
</script>

<div class="snapshot">
  {#if recording.stats?.median != null}
    {@const s = recording.stats}
    <div class="stats-grid">
      <div class="stat-box">
        <div class="stat-val" style="color:{s.median >= TARGET_LO ? 'var(--trans-blue)' : '#f39c12'}">
          {Math.round(s.median)}
        </div>
        <div class="stat-label">Median Pitch (Hz)</div>
      </div>
      <div class="stat-box">
        <div class="stat-val" style="color:{s.tgtPct >= 50 ? 'var(--trans-blue)' : '#f39c12'}">
          {s.tgtPct}%
        </div>
        <div class="stat-label">Time in Target Range</div>
      </div>
      <div class="stat-box">
        <div class="stat-val" style="font-size:1.25rem; color:var(--text)">
          {Math.round(s.pct10)}–{Math.round(s.pct90)}
        </div>
        <div class="stat-label">Pitch Range (10–90th %ile)</div>
      </div>
      <div class="stat-box">
        <div class="stat-val" style="color:{s.f2f1Ratio >= 1.3 ? 'var(--trans-blue)' : s.f2f1Ratio >= 0.9 ? 'var(--trans-pink)' : '#f39c12'}">
          {s.f2f1Ratio.toFixed(2)}×
        </div>
        <div class="stat-label">F2 / F1 Brightness</div>
        <div class="stat-sub">{s.ratioLabel}</div>
      </div>
    </div>
  {/if}

  {#if recording.pitchLog.length}
    <div class="sub-label">Pitch Over Time</div>
    <canvas bind:this={pitchCanvas} style="height:150px"></canvas>
  {/if}

  {#if recording.formantData.length}
    <div class="sub-label">F1 / F2 Formant Map</div>
    <canvas bind:this={specCanvas} style="height:220px"></canvas>
  {:else if !recording.stats?.median}
    <p class="no-data">This recording has no snapshot data. Delete it and re-save a new recording to see the full analysis.</p>
  {:else}
    <p class="no-data">No formant data available for this recording.</p>
  {/if}
</div>

<style>
  .snapshot { display: flex; flex-direction: column; gap: 0; }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.25rem;
  }
  .stat-box {
    background: #0d0d24;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.8rem 1rem;
    text-align: center;
  }
  .stat-val {
    font-size: 1.9rem;
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }
  .stat-label {
    font-size: 0.67rem;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.07em;
    margin-top: 0.3rem;
  }
  .stat-sub { font-size: 0.72rem; color: var(--muted); margin-top: 0.15rem; }

  .sub-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--muted);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    margin: 1rem 0 0.5rem;
  }

  canvas {
    display: block;
    width: 100%;
    border-radius: 8px;
    background: #0d0d24;
  }

  .no-data {
    color: var(--muted);
    font-size: 0.8rem;
    text-align: center;
    padding: 1rem 0;
  }
</style>
