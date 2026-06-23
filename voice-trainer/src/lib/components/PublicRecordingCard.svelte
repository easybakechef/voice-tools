<script lang="ts">
  import type { PublicRecording } from '$lib/data/recordings.js';
  import { TARGET_LO } from '$lib/audio/constants.js';
  import { listComments, addComment, deleteComment, authorLabel, type FeedbackComment } from '$lib/data/feedback.js';
  import CommentMenu from './CommentMenu.svelte';

  let { recording, isPlaying, onPlay }: {
    recording: PublicRecording;
    isPlaying: boolean;
    onPlay: (rec: PublicRecording) => void;
  } = $props();

  let comments = $state<FeedbackComment[]>([]);
  let loading  = $state(true);
  let draft    = $state('');
  let posting  = $state(false);
  let error    = $state('');

  $effect(() => {
    const id = recording.id;
    loading = true;
    listComments(id)
      .then((c) => { comments = c; })
      .catch((e) => { error = String(e); })
      .finally(() => { loading = false; });
  });

  async function post() {
    const body = draft.trim();
    if (!body || posting) return;
    posting = true;
    error = '';
    try {
      const c = await addComment(recording.id, body);
      comments = [...comments, c];
      draft = '';
    } catch (e) {
      error = String(e);
    } finally {
      posting = false;
    }
  }

  async function remove(c: FeedbackComment) {
    const prev = comments;
    comments = comments.filter((x) => x.id !== c.id); // optimistic
    try {
      await deleteComment(c.id);
    } catch (e) {
      comments = prev; // revert on failure
      error = String(e);
    }
  }

  function fmtDate(ts: number) {
    return new Date(ts).toLocaleString(undefined, {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
    });
  }
</script>

<div class="card">
  <div class="head">
    <div class="head-text">
      <div class="title">{recording.name}</div>
      <div class="sub">by Anon {recording.authorId.slice(0, 6)} · {fmtDate(recording.date)}</div>
    </div>
    <button class="play-btn" class:playing={isPlaying} onclick={() => onPlay(recording)}>
      {isPlaying ? '■ Stop' : '▶ Play'}
    </button>
  </div>

  {#if recording.stats?.median != null}
    {@const s = recording.stats}
    <div class="stats">
      <span class="stat" style="color:{s.median >= TARGET_LO ? 'var(--trans-blue)' : '#f39c12'}">
        {Math.round(s.median)} Hz median
      </span>
      <span class="stat">{s.tgtPct}% in target</span>
      <span class="stat">F2/F1 {s.f2f1Ratio.toFixed(2)}×</span>
    </div>
  {/if}

  <div class="thread">
    {#if loading}
      <p class="muted">Loading feedback…</p>
    {:else if comments.length}
      {#each comments as c (c.id)}
        <div class="comment" class:mine={c.mine}>
          <span class="c-author">{authorLabel(c)}</span>
          <span class="c-body">{c.body}</span>
          {#if c.mine}
            <CommentMenu onDelete={() => remove(c)} />
          {/if}
        </div>
      {/each}
    {:else}
      <p class="muted">No feedback yet — be the first.</p>
    {/if}
  </div>

  <div class="compose">
    <input
      class="c-input"
      bind:value={draft}
      placeholder="Leave feedback…"
      maxlength="2000"
      onkeydown={(e) => { if (e.key === 'Enter') post(); }}
    />
    <button class="post-btn" onclick={post} disabled={posting || !draft.trim()}>
      {posting ? '…' : 'Post'}
    </button>
  </div>
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .card {
    background: #12122a;
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1rem 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .head { display: flex; align-items: center; gap: 1rem; }
  .head-text { flex: 1; min-width: 0; }
  .title { font-weight: 700; font-size: 0.95rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sub { font-size: 0.72rem; color: var(--muted); margin-top: 0.15rem; }

  .play-btn {
    flex-shrink: 0;
    padding: 0.4rem 1rem;
    border-radius: 20px;
    border: 1px solid rgba(91,206,250,0.4);
    background: rgba(91,206,250,0.1);
    color: var(--trans-blue);
    font-size: 0.8rem;
    font-weight: 700;
    cursor: pointer;
  }
  .play-btn.playing { border-color: rgba(231,76,111,0.4); background: rgba(231,76,111,0.1); color: #e74c6f; }
  .play-btn:hover { filter: brightness(1.2); }

  .stats { display: flex; flex-wrap: wrap; gap: 0.75rem; font-size: 0.75rem; }
  .stat { color: var(--muted); font-variant-numeric: tabular-nums; }

  .thread { display: flex; flex-direction: column; gap: 0.4rem; }
  .comment { font-size: 0.84rem; line-height: 1.45; display: flex; align-items: baseline; gap: 0.4rem; }
  .c-author { font-weight: 600; color: var(--trans-blue); flex-shrink: 0; }
  .comment.mine .c-author { color: var(--trans-pink); }
  .c-body { color: var(--text); flex: 1; min-width: 0; }

  .compose { display: flex; gap: 0.5rem; }
  .c-input {
    flex: 1;
    background: #0d0d24;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.45rem 0.7rem;
    color: var(--text);
    font-size: 0.84rem;
  }
  .c-input:focus { outline: none; border-color: var(--trans-pink); }
  .post-btn {
    background: var(--trans-pink);
    color: #0d0d24;
    border: none;
    border-radius: 6px;
    padding: 0.45rem 1rem;
    font-weight: 700;
    font-size: 0.84rem;
    cursor: pointer;
  }
  .post-btn:disabled { opacity: 0.5; cursor: default; }

  .muted { color: var(--muted); font-size: 0.8rem; }
  .error { color: #e74c3c; font-size: 0.78rem; }
</style>
