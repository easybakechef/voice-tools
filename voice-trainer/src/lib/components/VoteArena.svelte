<script lang="ts">
  import { onMount } from 'svelte';
  import { engine } from '$lib/audio/engine.svelte.js';
  import { getAudioUrl, type PublicRecording } from '$lib/data/recordings.js';
  import { getSetItems, getMyVotes, recordVote, deleteVote, type ComparisonSet } from '$lib/data/ranking.js';

  let { set, onBack }: { set: ComparisonSet; onBack: () => void } = $props();

  type Pair = [PublicRecording, PublicRecording];

  let items     = $state<PublicRecording[]>([]);
  let allPairs: Pair[] = [];                              // every unordered pair (built once)
  let votedKeys = $state<Set<string>>(new Set());        // unordered keys already judged
  let history   = $state<{ voteId: string; pair: Pair }[]>([]); // this session, for undo
  let pair      = $state<Pair | null>(null);
  let done      = $state(false);
  let loading   = $state(true);
  let error     = $state('');
  let busy      = $state(false);
  let playingId = $state<string | null>(null);

  const totalPairs = $derived(allPairs.length);

  function keyOf(a: string, b: string) {
    return a < b ? `${a}|${b}` : `${b}|${a}`;
  }

  onMount(async () => {
    try {
      items = await getSetItems(set.id);
      for (let i = 0; i < items.length; i++)
        for (let j = i + 1; j < items.length; j++) allPairs.push([items[i], items[j]]);

      const mine = await getMyVotes(set.id);
      votedKeys = new Set(mine.map((v) => keyOf(v.a, v.b)));
      nextPair();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  $effect(() => {
    if (engine.activeType === null) playingId = null;
  });

  function remainingPairs(): Pair[] {
    return allPairs.filter(([a, b]) => !votedKeys.has(keyOf(a.id, b.id)));
  }

  function nextPair() {
    engine.stopAll();
    playingId = null;
    const remaining = remainingPairs();
    if (!remaining.length) { pair = null; done = true; return; }
    done = false;
    pair = remaining[Math.floor(Math.random() * remaining.length)];
  }

  async function play(rec: PublicRecording) {
    if (playingId === rec.id) { engine.stopAll(); playingId = null; return; }
    playingId = rec.id;
    const url = await getAudioUrl(rec.id);
    await engine.loadUrl(url, rec.name);
  }

  async function choose(winner: PublicRecording) {
    if (!pair || busy) return;
    busy = true; error = '';
    const [a, b] = pair;
    try {
      const voteId = await recordVote({ setId: set.id, recordingA: a.id, recordingB: b.id, winnerId: winner.id });
      votedKeys = new Set(votedKeys).add(keyOf(a.id, b.id));
      history = [...history, { voteId, pair: [a, b] }];
      nextPair();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  // Undo the most recent vote and re-show that pair to re-judge.
  async function undo() {
    if (!history.length || busy) return;
    busy = true; error = '';
    const last = history[history.length - 1];
    try {
      await deleteVote(last.voteId);
      const next = new Set(votedKeys);
      next.delete(keyOf(last.pair[0].id, last.pair[1].id));
      votedKeys = next;
      history = history.slice(0, -1);
      engine.stopAll();
      playingId = null;
      done = false;
      pair = last.pair;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="arena">
  <div class="head">
    <button class="back" onclick={onBack}>← Sets</button>
    <div class="title">{set.name}</div>
    <button class="undo" disabled={!history.length || busy} onclick={undo}>↶ Undo last</button>
  </div>

  {#if !loading && !error && totalPairs > 0}
    <div class="progress">
      <div class="progress-bar"><div class="progress-fill" style="width:{(votedKeys.size / totalPairs) * 100}%"></div></div>
      <span class="progress-text">{votedKeys.size} / {totalPairs} pairs ranked</span>
    </div>
  {/if}

  {#if loading}
    <p class="muted">Loading clips…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if totalPairs === 0}
    <p class="muted">This set needs at least 2 recordings to compare.</p>
  {:else if done}
    <div class="done">
      <div class="done-icon">✓</div>
      <p>You've ranked every pair in this set.</p>
      <p class="muted">Use “Undo last” to revisit your previous choice, or head back to the sets.</p>
      <button class="primary" onclick={onBack}>Back to sets</button>
    </div>
  {:else if pair}
    <p class="prompt">Which voice sounds <strong>more feminine</strong>?</p>
    <div class="pair">
      {#each pair as rec, idx (rec.id + idx)}
        <div class="choice">
          <div class="choice-label">{idx === 0 ? 'A' : 'B'}</div>
          <div class="choice-name">Anon {rec.authorId.slice(0, 6)}</div>
          {#if rec.stats?.median != null}
            <div class="choice-stat">{Math.round(rec.stats.median)} Hz median</div>
          {/if}
          <button class="play" class:playing={playingId === rec.id} onclick={() => play(rec)}>
            {playingId === rec.id ? '■ Stop' : '▶ Listen'}
          </button>
          <button class="pick" disabled={busy} onclick={() => choose(rec)}>More feminine</button>
        </div>
      {/each}
    </div>
    <div class="actions">
      <button class="skip" onclick={nextPair} disabled={busy}>Skip / too close →</button>
    </div>
  {/if}
</div>

<style>
  .arena { display: flex; flex-direction: column; gap: 1rem; }
  .head { display: flex; align-items: center; gap: 1rem; }
  .back, .undo {
    background: transparent; border: 1px solid var(--border); border-radius: 20px;
    color: var(--muted); font-size: 0.8rem; font-weight: 600; padding: 0.35rem 0.9rem; cursor: pointer;
  }
  .back:hover, .undo:hover:not(:disabled) { color: var(--text); }
  .undo:disabled { opacity: 0.4; cursor: default; }
  .title { flex: 1; font-weight: 700; font-size: 1rem; }

  .progress { display: flex; align-items: center; gap: 0.75rem; }
  .progress-bar { flex: 1; height: 5px; background: #12122a; border-radius: 3px; overflow: hidden; }
  .progress-fill { height: 100%; background: linear-gradient(90deg, var(--trans-blue), var(--trans-pink)); border-radius: 3px; transition: width 0.2s; }
  .progress-text { font-size: 0.72rem; color: var(--muted); white-space: nowrap; }

  .prompt { text-align: center; font-size: 1.05rem; margin: 0.5rem 0; }
  .prompt strong { color: var(--trans-pink); }

  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
  @media (max-width: 520px) { .pair { grid-template-columns: 1fr; } }

  .choice {
    background: #12122a; border: 1px solid var(--border); border-radius: 12px;
    padding: 1.25rem 1rem; display: flex; flex-direction: column; align-items: center; gap: 0.6rem;
  }
  .choice-label {
    width: 2rem; height: 2rem; border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    font-weight: 800; color: #0d0d24; background: var(--trans-blue);
  }
  .choice-name { font-size: 0.78rem; color: var(--muted); }
  .choice-stat { font-size: 0.78rem; color: var(--trans-blue); font-variant-numeric: tabular-nums; }

  .play {
    padding: 0.45rem 1.3rem; border-radius: 20px;
    border: 1px solid rgba(91,206,250,0.4); background: rgba(91,206,250,0.1);
    color: var(--trans-blue); font-size: 0.85rem; font-weight: 700; cursor: pointer;
  }
  .play.playing { border-color: rgba(231,76,111,0.4); background: rgba(231,76,111,0.1); color: #e74c6f; }
  .play:hover { filter: brightness(1.2); }

  .pick {
    margin-top: 0.3rem; width: 100%;
    padding: 0.6rem 1rem; border-radius: 8px; border: none;
    background: var(--trans-pink); color: #0d0d24; font-weight: 700; font-size: 0.9rem; cursor: pointer;
  }
  .pick:hover:not(:disabled) { filter: brightness(1.08); }
  .pick:disabled { opacity: 0.5; cursor: default; }

  .actions { display: flex; justify-content: center; }
  .skip { background: transparent; border: none; color: var(--muted); font-size: 0.82rem; cursor: pointer; padding: 0.3rem 0.8rem; }
  .skip:hover { color: var(--text); }

  .done { text-align: center; display: flex; flex-direction: column; align-items: center; gap: 0.6rem; padding: 1.5rem 0; }
  .done-icon {
    width: 3rem; height: 3rem; border-radius: 50%; display: flex; align-items: center; justify-content: center;
    font-size: 1.5rem; font-weight: 800; color: #0d0d24; background: var(--trans-blue);
  }
  .primary { margin-top: 0.5rem; border: none; border-radius: 8px; cursor: pointer; background: var(--trans-pink); color: #0d0d24; font-weight: 700; font-size: 0.85rem; padding: 0.5rem 1.2rem; }

  .muted { color: var(--muted); font-size: 0.85rem; text-align: center; }
  .error { color: #e74c3c; font-size: 0.82rem; text-align: center; }
</style>
