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
    try {
      phrases = await listPhrases();
      if (phrases.length) phraseIdx = Math.floor(Math.random() * phrases.length);
    }
    catch (e) { error = String(e); }
    finally { loading = false; }
  });

  // Pick a random phrase index, avoiding an immediate repeat when possible.
  function pickRandomIndex(): number {
    if (phrases.length <= 1) return 0;
    let i = Math.floor(Math.random() * phrases.length);
    if (i === phraseIdx) i = (i + 1) % phrases.length;
    return i;
  }

  function randomPhrase() {
    phraseIdx = pickRandomIndex();
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
      // load a fresh random phrase to keep contributing
      phraseIdx = pickRandomIndex();
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
      <div class="phrase-text">“{phrase.text}”</div>
      <button class="random" onclick={randomPhrase}>🎲 New phrase</button>
    </div>

    <p class="tip">
      💡 Try to keep your <strong>pitch</strong> about the same across both takes — only your
      <strong>resonance</strong> should change. The average pitch under each take helps you match
      them, so listeners judge resonance and don't confuse it with pitch.
    </p>

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

  .phrase-box { display: flex; flex-direction: column; align-items: center; gap: 0.75rem; margin: 1rem 0; }
  .phrase-text {
    width: 100%; text-align: center; font-size: 1.15rem; line-height: 1.5;
    background: #0d0d24; border: 1px solid var(--border); border-radius: 12px; padding: 1.1rem 1rem;
  }
  .random {
    border: 1px solid var(--border); border-radius: 20px; background: transparent;
    color: var(--muted); font-size: 0.82rem; font-weight: 600; padding: 0.4rem 1.1rem; cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }
  .random:hover { color: var(--text); border-color: var(--trans-pink); }

  .tip {
    font-size: 0.78rem;
    color: var(--muted);
    line-height: 1.55;
    background: rgba(91,206,250,0.06);
    border: 1px solid rgba(91,206,250,0.2);
    border-radius: 8px;
    padding: 0.65rem 0.85rem;
    margin: 0.9rem 0;
  }
  .tip strong { color: var(--text); }

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
