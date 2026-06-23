<script lang="ts">
  import { engine } from '$lib/audio/engine.svelte.js';
  import { CLIPS } from '$lib/audio/constants.js';

  let uploadName = $state('');

  async function handleUpload(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    uploadName = file.name;
    await engine.loadFile(file);
  }
</script>

<div class="card">
  <div class="card-label">Reference Voice Patterns</div>

  <div class="demo-grid">
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div
      class="demo-card fem"
      class:selected={engine.selectedPreset === 'fem'}
      role="button"
      tabindex="0"
      onclick={() => engine.selectPreset('fem')}
      onkeydown={e => e.key === 'Enter' && engine.selectPreset('fem')}
    >
      <h3>Feminine Voice — Karen Savage</h3>
      <div class="demo-source">{CLIPS.fem.source}</div>
      <p class="demo-desc">{CLIPS.fem.desc}</p>
      <button
        class="demo-btn"
        class:active={engine.activePreset === 'fem'}
        onclick={(e) => { e.stopPropagation(); engine.startClip('fem'); }}
      >
        {engine.activePreset === 'fem' ? '⏹ Stop' : '▶ Play 30s Clip'}
      </button>
    </div>

    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div
      class="demo-card masc"
      class:selected={engine.selectedPreset === 'masc'}
      role="button"
      tabindex="0"
      onclick={() => engine.selectPreset('masc')}
      onkeydown={e => e.key === 'Enter' && engine.selectPreset('masc')}
    >
      <h3>Masculine Voice — Bryan Ness</h3>
      <div class="demo-source">{CLIPS.masc.source}</div>
      <p class="demo-desc">{CLIPS.masc.desc}</p>
      <button
        class="demo-btn"
        class:active={engine.activePreset === 'masc'}
        onclick={(e) => { e.stopPropagation(); engine.startClip('masc'); }}
      >
        {engine.activePreset === 'masc' ? '⏹ Stop' : '▶ Play 30s Clip'}
      </button>
    </div>
  </div>

  <div class="upload-row">
    <label class="upload-lbl">
      📂 Upload Audio
      <input type="file" accept="audio/*" style="display:none" onchange={handleUpload} />
    </label>
    <span class="upload-name">{uploadName || 'No file selected'}</span>
  </div>

  {#if engine.activeType === 'clip' || engine.activeType === 'file'}
    <div class="pb-bar">
      <span class="pb-label">Now playing</span>
      <span class="pb-name">
        {engine.activeType === 'file' ? uploadName
          : engine.activePreset === 'fem' ? CLIPS.fem.label : CLIPS.masc.label}
      </span>
      <div class="pb-track-wrap">
        <div class="pb-track">
          <div class="pb-fill" style="width: {engine.playbackProgress * 100}%"></div>
        </div>
      </div>
      <span class="pb-time">{engine.playbackTime}</span>
      <button class="stop-btn" onclick={() => engine.stopAll()}>⏹ Stop</button>
    </div>
  {/if}
</div>

<style>
  .demo-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  @media (max-width: 520px) { .demo-grid { grid-template-columns: 1fr; } }

  .demo-card {
    background: #12122a;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1rem 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .demo-card:hover { background: #15153a; }
  .demo-card.fem.selected  { border-color: var(--trans-pink); background: rgba(245,169,184,0.06); }
  .demo-card.masc.selected { border-color: var(--trans-blue); background: rgba(91,206,250,0.06); }
  .demo-card h3 { font-size: 0.95rem; font-weight: 700; }
  .demo-card.fem h3  { color: var(--trans-pink); }
  .demo-card.masc h3 { color: var(--trans-blue); }
  .demo-source { font-size: 0.72rem; color: var(--muted); }
  .demo-desc   { font-size: 0.78rem; color: var(--muted); line-height: 1.55; flex: 1; }

  .demo-btn {
    margin-top: 0.5rem;
    padding: 0.45rem 1.1rem;
    border-radius: 20px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    align-self: flex-start;
  }
  .demo-card.fem .demo-btn {
    background: rgba(245,169,184,0.15);
    color: var(--trans-pink);
    border: 1px solid rgba(245,169,184,0.4);
  }
  .demo-card.masc .demo-btn {
    background: rgba(91,206,250,0.15);
    color: var(--trans-blue);
    border: 1px solid rgba(91,206,250,0.4);
  }
  .demo-btn:hover { filter: brightness(1.25); }
  .demo-btn.active { filter: brightness(1.4); box-shadow: 0 0 10px currentColor; }

  .upload-row {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    padding-top: 0.85rem;
    border-top: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .upload-lbl {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.45rem 1.1rem;
    border-radius: 20px;
    background: rgba(255,255,255,0.06);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
  }
  .upload-lbl:hover { background: rgba(255,255,255,0.11); }
  .upload-name { font-size: 0.78rem; color: var(--muted); }

  .pb-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding-top: 0.85rem;
    border-top: 1px solid var(--border);
    flex-wrap: wrap;
    margin-top: 0.85rem;
  }
  .pb-label { font-size: 0.75rem; color: var(--muted); white-space: nowrap; }
  .pb-name  { font-size: 0.82rem; font-weight: 600; white-space: nowrap; }
  .pb-track-wrap { flex: 1; min-width: 120px; }
  .pb-track {
    height: 6px;
    background: #12122a;
    border-radius: 3px;
    overflow: hidden;
  }
  .pb-fill {
    height: 100%;
    border-radius: 3px;
    background: linear-gradient(90deg, var(--trans-blue), var(--trans-pink));
    transition: width 0.25s linear;
  }
  .pb-time { font-size: 0.72rem; color: var(--muted); white-space: nowrap; }
  .stop-btn {
    padding: 0.3rem 0.9rem;
    border-radius: 20px;
    border: 1px solid rgba(231,76,111,0.4);
    background: rgba(231,76,111,0.12);
    color: #e74c6f;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .stop-btn:hover { background: rgba(231,76,111,0.25); }
</style>
