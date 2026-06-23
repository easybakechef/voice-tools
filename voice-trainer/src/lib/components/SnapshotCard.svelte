<script lang="ts">
  import { engine } from '$lib/audio/engine.svelte.js';
  import { TARGET_LO } from '$lib/audio/constants.js';
  import {
    computeSnapshotStats,
    renderPitchTimeline,
    renderFormantPlot,
    resizeDPR,
    extractFormantData,
  } from '$lib/audio/drawing.js';
  import { saveRecording } from '$lib/data/recordings.js';
  import type { PitchPoint } from '$lib/audio/types.js';

  let { clipMode = false }: { clipMode?: boolean } = $props();

  let pitchCanvas     = $state<HTMLCanvasElement | null>(null);
  let specCanvas      = $state<HTMLCanvasElement | null>(null);
  let saveName        = $state('');
  let savedId         = $state<string | null>(null);
  let saving          = $state(false);

  // ── Inline playback of the recorded blob (no engine involvement) ──────────
  let audioEl          = $state<HTMLAudioElement | null>(null);
  let blobUrl          = $state<string | null>(null);
  let isPlayingBack    = $state(false);
  let playbackProgress = $state(0);

  $effect(() => {
    const blob = engine.recordingBlob;
    const url  = blob ? URL.createObjectURL(blob) : null;
    blobUrl          = url;
    isPlayingBack    = false;
    playbackProgress = 0;
    if (audioEl) { audioEl.pause(); audioEl.currentTime = 0; }
    // Revoke the URL created in THIS run when the effect re-runs or the component unmounts.
    // Capturing `url` (local) instead of reading reactive `blobUrl` prevents an infinite loop.
    return () => { if (url) URL.revokeObjectURL(url); };
  });

  function togglePlayback() {
    if (!audioEl || !blobUrl) return;
    if (isPlayingBack) {
      audioEl.pause();
      audioEl.currentTime = 0;
      isPlayingBack    = false;
      playbackProgress = 0;
    } else {
      audioEl.play();
      isPlayingBack = true;
    }
  }

  type SnapSource = { pitchLog: PitchPoint[]; specFrameStore: Float32Array[]; sampleRate: number };

  // Derive the data source: fresh engine data when snapshotReady, else cached clip data.
  const snapSource = $derived.by((): SnapSource | null => {
    if (engine.snapshotReady) {
      return { pitchLog: engine.pitchLog, specFrameStore: engine.specFrameStore, sampleRate: engine.sampleRate };
    }
    if (clipMode) {
      const _v = engine.clipSnapshotsVersion; // reactive dep for cache updates
      const key = engine.selectedPreset;
      if (key) {
        const cached = engine.clipSnapshots[key];
        if (cached) return { ...cached, sampleRate: engine.sampleRate };
      }
    }
    return null;
  });

  const stats = $derived(
    snapSource?.pitchLog.length
      ? computeSnapshotStats(snapSource.pitchLog, snapSource.specFrameStore, snapSource.sampleRate)
      : null
  );

  $effect(() => {
    if (!snapSource || !pitchCanvas || !specCanvas) return;
    if (engine.snapshotReady) {
      savedId  = null;
      saving   = false;
      saveName = new Date().toLocaleString(undefined, {
        month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
      });
    }
    resizeDPR(pitchCanvas);
    resizeDPR(specCanvas);
    renderPitchTimeline(pitchCanvas, snapSource.pitchLog);
    renderFormantPlot(specCanvas, snapSource.specFrameStore, snapSource.sampleRate);
  });

  async function handleSave() {
    if (!engine.recordingBlob || !stats) return;
    saving = true;
    try {
      savedId = await saveRecording({
        name:        saveName || 'Untitled',
        date:        Date.now(),
        duration:    engine.pitchLog.at(-1)?.t ?? 0,
        medianPitch: stats.median,
        blob:        engine.recordingBlob,
        pitchLog:    [...engine.pitchLog],
        formantData: extractFormantData(engine.specFrameStore, engine.sampleRate),
        stats,
      });
    } catch (e) {
      console.error('Save failed', e);
    }
    saving = false;
  }
</script>

{#if snapSource}
  <div class="card">
    <div class="card-label">Sample Snapshot</div>

    {#if stats}
      <div class="stats-grid">
        <div class="stat-box">
          <div class="stat-val" style="color:{stats.median >= TARGET_LO ? 'var(--trans-blue)' : '#f39c12'}">
            {Math.round(stats.median)}
          </div>
          <div class="stat-label">Median Pitch (Hz)</div>
        </div>
        <div class="stat-box">
          <div class="stat-val" style="color:{stats.tgtPct >= 50 ? 'var(--trans-blue)' : '#f39c12'}">
            {stats.tgtPct}%
          </div>
          <div class="stat-label">Time in Target Range</div>
        </div>
        <div class="stat-box">
          <div class="stat-val" style="font-size:1.25rem; color:var(--text)">
            {Math.round(stats.pct10)}–{Math.round(stats.pct90)}
          </div>
          <div class="stat-label">Pitch Range (10–90th %ile)</div>
        </div>
        <div class="stat-box">
          <div class="stat-val" style="color:{stats.f2f1Ratio >= 1.3 ? 'var(--trans-blue)' : stats.f2f1Ratio >= 0.9 ? 'var(--trans-pink)' : '#f39c12'}">
            {stats.f2f1Ratio.toFixed(2)}×
          </div>
          <div class="stat-label">F2 / F1 Brightness</div>
          <div class="stat-sub">{stats.ratioLabel}</div>
        </div>
      </div>
    {/if}

    <div class="sub-label">Pitch Over Time</div>
    <canvas bind:this={pitchCanvas} style="height:150px"></canvas>

    <div class="sub-label">F1 / F2 Formant Map</div>
    <canvas bind:this={specCanvas} style="height:220px"></canvas>

    {#if engine.recordingBlob}
      <!-- svelte-ignore a11y_media_has_caption -->
      <audio
        bind:this={audioEl}
        src={blobUrl}
        ontimeupdate={() => { if (audioEl?.duration) playbackProgress = audioEl.currentTime / audioEl.duration; }}
        onended={() => { isPlayingBack = false; playbackProgress = 0; if (audioEl) audioEl.currentTime = 0; }}
      ></audio>

      <div class="playback-row">
        <button class="pb-btn" class:playing={isPlayingBack} onclick={togglePlayback}>
          {isPlayingBack ? '■ Stop' : '▶ Play Back'}
        </button>
        <div class="pb-track">
          <div class="pb-fill" style="width:{playbackProgress * 100}%"></div>
        </div>
      </div>
    {/if}

    {#if engine.recordingBlob && engine.snapshotReady && !engine.isRecordingPaused}
      {#if savedId}
        <div class="save-confirm">Saved to library ✓</div>
      {:else}
        <div class="save-row">
          <input class="save-input" bind:value={saveName} placeholder="Recording name…" />
          <button class="save-btn" onclick={handleSave} disabled={saving}>
            {saving ? 'Saving…' : 'Save to Library'}
          </button>
        </div>
      {/if}
    {/if}
  </div>
{:else if clipMode && engine.selectedPreset}
  <div class="card">
    <div class="card-label">Clip Analysis</div>
    {#if engine.clipAnalysisState[engine.selectedPreset] === 'loading'}
      <p class="no-data loading">Analyzing audio…</p>
    {:else if engine.clipAnalysisState[engine.selectedPreset] === 'error'}
      <p class="no-data">Analysis failed — play the clip to generate its snapshot.</p>
    {:else}
      <p class="no-data">Play this clip to generate its analysis.</p>
    {/if}
  </div>
{/if}

<style>
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(145px, 1fr));
    gap: 0.75rem;
    margin-bottom: 0.25rem;
  }
  .stat-box {
    background: #12122a;
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
    background: #12122a;
  }

  .playback-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-top: 1rem;
  }
  .pb-btn {
    flex-shrink: 0;
    padding: 0.4rem 1rem;
    border-radius: 20px;
    border: 1px solid rgba(91,206,250,0.4);
    background: rgba(91,206,250,0.08);
    color: var(--trans-blue);
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    white-space: nowrap;
  }
  .pb-btn.playing {
    border-color: rgba(231,76,111,0.4);
    background: rgba(231,76,111,0.08);
    color: #e74c6f;
  }
  .pb-btn:hover { filter: brightness(1.25); }
  .pb-track {
    flex: 1;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .pb-fill {
    height: 100%;
    background: var(--trans-blue);
    border-radius: 2px;
    transition: width 0.1s linear;
  }

  .save-row {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
    align-items: center;
  }
  .save-input {
    flex: 1;
    background: #12122a;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.45rem 0.75rem;
    color: var(--text);
    font-size: 0.875rem;
  }
  .save-input:focus { outline: none; border-color: var(--trans-pink); }
  .save-btn {
    background: var(--trans-pink);
    color: #0d0d24;
    border: none;
    border-radius: 6px;
    padding: 0.45rem 1rem;
    font-weight: 600;
    font-size: 0.875rem;
    cursor: pointer;
    white-space: nowrap;
  }
  .save-btn:disabled { opacity: 0.6; cursor: default; }
  .save-confirm {
    margin-top: 1rem;
    text-align: center;
    color: var(--trans-blue);
    font-size: 0.875rem;
    font-weight: 600;
  }
  .no-data {
    color: var(--muted);
    font-size: 0.85rem;
    text-align: center;
    padding: 1.5rem 0;
  }
  .no-data.loading {
    animation: pulse 1.4s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.4; }
  }
</style>
