<script lang="ts">
  import { engine } from '$lib/audio/engine.svelte.js';
</script>

<div class="card">
  <div class="card-label">Recording</div>
  <div class="controls">

    {#if engine.isRecording}
      <!-- Actively recording -->
      <button class="record-btn recording" onclick={() => engine.stopRecording()}>
        <span class="dot"></span>Stop Recording
      </button>
      <button class="pause-btn" onclick={() => engine.pauseForReview()}>
        ⏸ Pause &amp; Review
      </button>

    {:else if engine.isRecordingPaused}
      <!-- Paused for review -->
      <button class="resume-btn" onclick={() => engine.resumeRecording()}>
        ● Resume Recording
      </button>
      <button class="stop-finish-btn" onclick={() => engine.stopRecording()}>
        ■ Stop &amp; Finish
      </button>

    {:else}
      <!-- Idle -->
      <button class="record-btn" onclick={() => engine.startRecording()}>
        <span class="dot"></span>Start Recording
      </button>
    {/if}

    <span class="status-msg">{engine.statusMsg}</span>

    <button
      class="monitor-btn"
      class:on={engine.monitorEnabled}
      onclick={() => engine.toggleMonitor()}
      title="Hear your voice through speakers"
    >
      {engine.monitorEnabled ? '🔊 Speaker On' : '🔇 Speaker Off'}
    </button>
  </div>
</div>

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  /* ── Start / Stop button ── */
  .record-btn {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.75rem 2rem;
    border: none;
    border-radius: 50px;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    background: linear-gradient(135deg, var(--trans-blue), var(--trans-pink));
    color: #fff;
    transition: transform 0.1s;
  }
  .record-btn:hover { transform: scale(1.03); }
  .record-btn.recording {
    background: linear-gradient(135deg, #e74c6f, #c0392b);
    animation: pulse 1.4s ease-out infinite;
  }
  @keyframes pulse {
    0%   { box-shadow: 0 0 0 0   rgba(231,76,111,0.5); }
    70%  { box-shadow: 0 0 0 14px rgba(231,76,111,0); }
    100% { box-shadow: 0 0 0 0   rgba(231,76,111,0); }
  }
  .dot {
    width: 10px; height: 10px;
    border-radius: 50%;
    background: #fff;
    flex-shrink: 0;
  }
  .record-btn.recording .dot { animation: blink 1s step-start infinite; }
  @keyframes blink { 50% { opacity: 0; } }

  /* ── Pause & Review ── */
  .pause-btn {
    padding: 0.55rem 1.2rem;
    border-radius: 50px;
    border: 1px solid rgba(245,169,184,0.4);
    background: rgba(245,169,184,0.08);
    color: var(--trans-pink);
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }
  .pause-btn:hover { background: rgba(245,169,184,0.18); }

  /* ── Resume ── */
  .resume-btn {
    padding: 0.75rem 2rem;
    border-radius: 50px;
    border: none;
    background: linear-gradient(135deg, var(--trans-blue), var(--trans-pink));
    color: #fff;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: transform 0.1s;
  }
  .resume-btn:hover { transform: scale(1.03); }

  /* ── Stop & Finish ── */
  .stop-finish-btn {
    padding: 0.55rem 1.2rem;
    border-radius: 50px;
    border: 1px solid rgba(231,76,111,0.4);
    background: rgba(231,76,111,0.08);
    color: #e74c6f;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }
  .stop-finish-btn:hover { background: rgba(231,76,111,0.18); }

  .status-msg { color: var(--muted); font-size: 0.9rem; }

  .monitor-btn {
    margin-left: auto;
    padding: 0.45rem 1rem;
    border-radius: 20px;
    border: 1px solid var(--border);
    background: transparent;
    color: var(--muted);
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
  }
  .monitor-btn.on {
    border-color: rgba(91,206,250,0.5);
    color: var(--trans-blue);
    background: rgba(91,206,250,0.1);
  }
</style>
