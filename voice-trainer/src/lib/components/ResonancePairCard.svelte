<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getMyResonanceVote, recordResonanceVote, getResonanceStats,
    listPairComments, addPairComment, deletePairComment, pairAuthorLabel,
    type ResonancePair, type ResonanceSample, type ResonanceStats, type PairComment,
  } from '$lib/data/resonance.js';
  import CommentMenu from './CommentMenu.svelte';

  let { pair, playingSampleId, onPlay }: {
    pair: ResonancePair;
    playingSampleId: string | null;
    onPlay: (s: ResonanceSample) => void;
  } = $props();

  // Random A/B order, fixed for this card instance so labels stay hidden.
  // (The card is keyed by pair.id, so capturing the initial prop is intentional.)
  // svelte-ignore state_referenced_locally
  const order: ResonanceSample[] = [...pair.samples].sort(() => Math.random() - 0.5);

  let chosenId  = $state<string | null>(null);
  let revealed  = $state(false);
  let stats     = $state<ResonanceStats | null>(null);
  let busy      = $state(false);
  let error     = $state('');

  let comments = $state<PairComment[]>([]);
  let draft    = $state('');
  let posting  = $state(false);

  const chosen   = $derived(order.find((s) => s.id === chosenId) ?? null);
  const agreed   = $derived(chosen?.label === 'bright'); // bright take is the "correct" brighter one

  onMount(async () => {
    try {
      const prior = await getMyResonanceVote(pair.id);
      if (prior) { chosenId = prior; revealed = true; stats = await getResonanceStats(pair.id); }
      comments = await listPairComments(pair.id);
    } catch (e) { error = String(e); }
  });

  async function choose(s: ResonanceSample) {
    if (revealed || busy) return;
    busy = true; error = '';
    try {
      await recordResonanceVote(pair.id, s.id);
      chosenId = s.id;
      revealed = true;
      stats = await getResonanceStats(pair.id);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function pct(n: number) {
    if (!stats || stats.total === 0) return 0;
    return Math.round((n / stats.total) * 100);
  }

  async function post() {
    const body = draft.trim();
    if (!body || posting) return;
    posting = true; error = '';
    try {
      const c = await addPairComment(pair.id, body);
      comments = [...comments, c];
      draft = '';
    } catch (e) { error = String(e); } finally { posting = false; }
  }

  async function removeComment(c: PairComment) {
    const prev = comments;
    comments = comments.filter((x) => x.id !== c.id);
    try { await deletePairComment(c.id); }
    catch (e) { comments = prev; error = String(e); }
  }
</script>

<div class="card">
  <div class="phrase">“{pair.phrase}”</div>
  <div class="prompt">{revealed ? 'You picked the brighter take:' : 'Which take sounds brighter?'}</div>

  <div class="takes">
    {#each order as s, i (s.id)}
      <div class="take" class:chosen={chosenId === s.id} class:reveal={revealed}>
        <div class="take-head">
          <span class="ab">{i === 0 ? 'A' : 'B'}</span>
          {#if revealed}
            <span class="label {s.label}">{s.label === 'bright' ? 'Bright' : 'Deep'}</span>
          {/if}
        </div>
        <button class="play" class:playing={playingSampleId === s.id} onclick={() => onPlay(s)}>
          {playingSampleId === s.id ? '■ Stop' : '▶ Listen'}
        </button>
        {#if !revealed}
          <button class="pick" disabled={busy} onclick={() => choose(s)}>Brighter</button>
        {:else if chosenId === s.id}
          <div class="your-pick">Your pick</div>
        {/if}
      </div>
    {/each}
  </div>

  {#if revealed}
    <div class="result" class:ok={agreed}>
      You chose the take the speaker labeled <strong>{chosen?.label === 'bright' ? 'Bright' : 'Deep'}</strong>
      — {agreed ? 'you agreed with their labelling ✓' : 'they labelled the other take brighter.'}
    </div>

    {#if stats && stats.total > 0}
      <div class="stats">
        <div class="stat-label">How {stats.total} {stats.total === 1 ? 'person' : 'people'} voted</div>
        <div class="bar-row">
          <span class="bar-name bright">Bright take</span>
          <div class="bar"><div class="fill bright" style="width:{pct(stats.bright)}%"></div></div>
          <span class="bar-val">{pct(stats.bright)}% ({stats.bright})</span>
        </div>
        <div class="bar-row">
          <span class="bar-name deep">Deep take</span>
          <div class="bar"><div class="fill deep" style="width:{pct(stats.deep)}%"></div></div>
          <span class="bar-val">{pct(stats.deep)}% ({stats.deep})</span>
        </div>
      </div>
    {/if}

    <div class="thread">
      {#if comments.length}
        {#each comments as c (c.id)}
          <div class="comment">
            <span class="c-author">{pairAuthorLabel(c)}</span>
            <span class="c-body">{c.body}</span>
            {#if c.mine}<CommentMenu onDelete={() => removeComment(c)} />{/if}
          </div>
        {/each}
      {:else}
        <p class="muted">No comments yet — share what you heard.</p>
      {/if}
    </div>
    <div class="compose">
      <input class="c-input" bind:value={draft} placeholder="Leave a comment…" maxlength="2000"
        onkeydown={(e) => { if (e.key === 'Enter') post(); }} />
      <button class="post-btn" onclick={post} disabled={posting || !draft.trim()}>{posting ? '…' : 'Post'}</button>
    </div>
  {/if}

  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .card { background: #12122a; border: 1px solid var(--border); border-radius: 12px; padding: 1.1rem 1.2rem; display: flex; flex-direction: column; gap: 0.85rem; }
  .phrase { font-size: 0.95rem; font-weight: 600; }
  .prompt { text-align: center; font-size: 0.92rem; color: var(--muted); }

  .takes { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
  @media (max-width: 520px) { .takes { grid-template-columns: 1fr; } }
  .take { background: #0d0d24; border: 1px solid var(--border); border-radius: 10px; padding: 1rem; display: flex; flex-direction: column; align-items: center; gap: 0.6rem; }
  .take.chosen { border-color: var(--trans-pink); box-shadow: 0 0 0 1px var(--trans-pink); }
  .take-head { display: flex; align-items: center; gap: 0.5rem; }
  .ab { width: 1.8rem; height: 1.8rem; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-weight: 800; color: #0d0d24; background: var(--text); }
  .label { font-size: 0.72rem; font-weight: 800; padding: 0.15rem 0.5rem; border-radius: 10px; }
  .label.bright { color: var(--trans-pink); background: rgba(245,169,184,0.15); }
  .label.deep   { color: #9b8cff; background: rgba(125,108,255,0.15); }

  .play { padding: 0.45rem 1.2rem; border-radius: 20px; border: 1px solid rgba(91,206,250,0.4); background: rgba(91,206,250,0.1); color: var(--trans-blue); font-size: 0.85rem; font-weight: 700; cursor: pointer; }
  .play.playing { border-color: rgba(231,76,111,0.4); background: rgba(231,76,111,0.1); color: #e74c6f; }
  .pick { width: 100%; padding: 0.55rem; border-radius: 8px; border: none; background: var(--trans-pink); color: #0d0d24; font-weight: 700; font-size: 0.88rem; cursor: pointer; }
  .pick:disabled { opacity: 0.5; cursor: default; }
  .your-pick { font-size: 0.76rem; color: var(--trans-pink); font-weight: 700; }

  .result { font-size: 0.85rem; line-height: 1.5; background: rgba(231,76,111,0.08); border: 1px solid rgba(231,76,111,0.25); border-radius: 8px; padding: 0.6rem 0.8rem; }
  .result.ok { background: rgba(91,206,250,0.08); border-color: rgba(91,206,250,0.3); }

  .stats { display: flex; flex-direction: column; gap: 0.4rem; }
  .stat-label { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); }
  .bar-row { display: flex; align-items: center; gap: 0.6rem; font-size: 0.76rem; }
  .bar-name { width: 5.5rem; flex-shrink: 0; font-weight: 600; }
  .bar-name.bright { color: var(--trans-pink); }
  .bar-name.deep { color: #9b8cff; }
  .bar { flex: 1; height: 0.9rem; background: #0d0d24; border-radius: 5px; overflow: hidden; }
  .fill { height: 100%; border-radius: 5px; }
  .fill.bright { background: var(--trans-pink); }
  .fill.deep { background: #7d6cff; }
  .bar-val { width: 4.5rem; text-align: right; color: var(--muted); font-variant-numeric: tabular-nums; }

  .thread { display: flex; flex-direction: column; gap: 0.4rem; }
  .comment { font-size: 0.84rem; line-height: 1.45; display: flex; align-items: baseline; gap: 0.4rem; }
  .c-author { font-weight: 600; color: var(--trans-blue); flex-shrink: 0; }
  .c-body { color: var(--text); flex: 1; min-width: 0; }
  .compose { display: flex; gap: 0.5rem; }
  .c-input { flex: 1; background: #0d0d24; border: 1px solid var(--border); border-radius: 6px; padding: 0.45rem 0.7rem; color: var(--text); font-size: 0.84rem; }
  .c-input:focus { outline: none; border-color: var(--trans-pink); }
  .post-btn { background: var(--trans-pink); color: #0d0d24; border: none; border-radius: 6px; padding: 0.45rem 1rem; font-weight: 700; font-size: 0.84rem; cursor: pointer; }
  .post-btn:disabled { opacity: 0.5; cursor: default; }

  .muted { color: var(--muted); font-size: 0.8rem; }
  .error { color: #e74c3c; font-size: 0.78rem; }
</style>
