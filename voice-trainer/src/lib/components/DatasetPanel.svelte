<script lang="ts">
  import { onMount } from 'svelte';
  import { listPhrases, submitPair, type SamplePhrase } from '$lib/data/dataset.js';
  import SampleRecorder from './SampleRecorder.svelte';
  import DatasetLibrary from './DatasetLibrary.svelte';

  let mode = $state<'record' | 'library'>('record');

  let phrases   = $state<SamplePhrase[]>([]);
  let phraseIdx = $state(0);
  let loading   = $state(true);
  let error     = $state('');

  let deepBlob   = $state<Blob | null>(null);
  let brightBlob = $state<Blob | null>(null);
  let resetToken = $state(0); // bump to remount both recorders fresh
  let submitting = $state(false);
  let justSaved  = $state(false);

  const phrase   = $derived(phrases[phraseIdx] ?? null);
  const canSubmit = $derived(!!deepBlob && !!brightBlob && !!phrase && !submitting);

  onMount(async () => {
    try { phrases = await listPhrases(); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  });

  function changePhrase(delta: number) {
    if (!phrases.length) return;
    phraseIdx = (phraseIdx + delta + phrases.length) % phrases.length;
    resetTakes();
  }

  function resetTakes() {
    deepBlob = null;
    brightBlob = null;
    resetToken += 1;
    justSaved = false;
  }

  async function submit() {
    if (!canSubmit || !phrase || !deepBlob || !brightBlob) return;
    submitting = true; error = '';
    try {
      await submitPair(phrase.id, deepBlob, brightBlob);
      resetTakes();
      justSaved = true;
      // advance to the next phrase to keep contributing
      if (phrases.length > 1) phraseIdx = (phraseIdx + 1) % phrases.length;
    } catch (e) {
      error = String(e);
    } finally {
      submitting = false;
    }
  }
</script>

<div class="card">
  <div class="card-head">
    <div>
      <div class="card-label">Resonance Recording</div>
      <p class="intro">Read the sentence twice — once with deep resonance, once bright — then submit both takes. You can publish a pair for community review from “My recordings”.</p>
    </div>
    <div class="seg">
      <button class:active={mode === 'record'} onclick={() => (mode = 'record')}>Record</button>
      <button class:active={mode === 'library'} onclick={() => (mode = 'library')}>My recordings</button>
    </div>
  </div>

  {#if mode === 'library'}
    <DatasetLibrary />
  {:else if loading}
    <p class="muted">Loading phrases…</p>
  {:else if error && !phrases.length}
    <p class="error">{error}</p>
  {:else if !phrase}
    <p class="muted">No sample phrases are available yet.</p>
  {:else}
    <div class="phrase-box">
      <button class="nav" onclick={() => changePhrase(-1)} title="Previous phrase">‹</button>
      <div class="phrase-text">
        <div class="phrase-eyebrow">Phrase {phraseIdx + 1} of {phrases.length}</div>
        “{phrase.text}”
      </div>
      <button class="nav" onclick={() => changePhrase(1)} title="Next phrase">›</button>
    </div>

    {#key resetToken}
      <div class="pair-grid">
        <SampleRecorder
          title="Deep / low resonance"
          hint="Read it with a darker, lower, chestier resonance."
          accent="deep"
          onBlob={(b) => (deepBlob = b)}
        />
        <SampleRecorder
          title="Bright resonance"
          hint="Read it again with a brighter, more forward resonance."
          accent="bright"
          onBlob={(b) => (brightBlob = b)}
        />
      </div>
    {/key}

    {#if error}<p class="error">{error}</p>{/if}
    {#if justSaved}<p class="saved">✓ Pair submitted — thank you! Next phrase loaded.</p>{/if}

    <div class="submit-row">
      <span class="hint">
        {deepBlob ? '✓' : '○'} Deep &nbsp; {brightBlob ? '✓' : '○'} Bright
      </span>
      <button class="submit" disabled={!canSubmit} onclick={submit}>
        {submitting ? 'Submitting…' : 'Submit both takes'}
      </button>
    </div>
  {/if}
</div>

<style>
  .card-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
  .intro { font-size: 0.82rem; color: var(--muted); margin: 0.25rem 0 0; }

  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 20px; overflow: hidden; flex-shrink: 0; }
  .seg button { background: transparent; border: none; color: var(--muted); font-size: 0.8rem; font-weight: 600; padding: 0.4rem 0.9rem; cursor: pointer; }
  .seg button.active { background: var(--trans-pink); color: #0d0d24; }

  .phrase-box { display: flex; align-items: stretch; gap: 0.75rem; margin: 1rem 0; }
  .phrase-text {
    flex: 1; text-align: center; font-size: 1.15rem; line-height: 1.5;
    background: #0d0d24; border: 1px solid var(--border); border-radius: 12px; padding: 1.1rem 1rem;
  }
  .phrase-eyebrow { font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); margin-bottom: 0.4rem; }
  .nav {
    flex-shrink: 0; width: 2.5rem; border: 1px solid var(--border); border-radius: 12px;
    background: transparent; color: var(--muted); font-size: 1.4rem; cursor: pointer;
  }
  .nav:hover { color: var(--text); }

  .pair-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
  @media (max-width: 560px) { .pair-grid { grid-template-columns: 1fr; } }

  .submit-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-top: 1.1rem; flex-wrap: wrap; }
  .hint { font-size: 0.85rem; color: var(--muted); font-variant-numeric: tabular-nums; }
  .submit {
    border: none; border-radius: 8px; cursor: pointer;
    background: var(--trans-pink); color: #0d0d24; font-weight: 700; font-size: 0.9rem; padding: 0.6rem 1.4rem;
  }
  .submit:disabled { opacity: 0.45; cursor: default; }
  .submit:hover:not(:disabled) { filter: brightness(1.08); }

  .saved { color: var(--trans-blue); font-size: 0.85rem; font-weight: 600; margin-top: 0.75rem; }
  .muted { color: var(--muted); font-size: 0.85rem; line-height: 1.6; }
  .error { color: #e74c3c; font-size: 0.85rem; margin-top: 0.5rem; }
</style>
