<script lang="ts">
  import type { SavedRecording, Visibility } from '$lib/data/recordings.js';
  import { setRecordingVisibility } from '$lib/data/recordings.js';
  import { listComments, deleteComment, authorLabel, type FeedbackComment } from '$lib/data/feedback.js';
  import CommentMenu from './CommentMenu.svelte';

  let { recording, onVisibilityChange }: {
    recording: SavedRecording;
    onVisibilityChange: (v: Visibility) => void;
  } = $props();

  let comments = $state<FeedbackComment[]>([]);
  let loading  = $state(false);
  let busy     = $state(false);
  let error    = $state('');

  const isPublic = $derived(recording.visibility === 'public');

  // (Re)load received feedback whenever the selected recording — or its
  // visibility — changes, but only when it's public (private has no readers).
  $effect(() => {
    const id  = recording.id;
    const pub = recording.visibility === 'public';
    if (!pub) { comments = []; return; }
    loading = true;
    listComments(id)
      .then((c) => { comments = c; })
      .catch((e) => { error = String(e); })
      .finally(() => { loading = false; });
  });

  // Owner moderation: on your own recording you may remove any comment.
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

  async function toggle() {
    busy = true;
    error = '';
    const next: Visibility = isPublic ? 'private' : 'public';
    try {
      await setRecordingVisibility(recording.id, next);
      onVisibilityChange(next);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function fmtDate(ts: number) {
    return new Date(ts).toLocaleString(undefined, {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
    });
  }
</script>

<div class="feedback">
  <div class="share-row">
    <div class="share-text">
      <div class="share-title">
        {isPublic ? '🌐 Shared for feedback' : '🔒 Private'}
      </div>
      <div class="share-sub">
        {isPublic
          ? 'This recording is visible on the community feed. Others can play it and leave feedback.'
          : 'Only you can see this. Make it public to receive feedback from the community.'}
      </div>
    </div>
    <button class="share-btn" class:on={isPublic} onclick={toggle} disabled={busy}>
      {busy ? '…' : isPublic ? 'Make Private' : 'Make Public'}
    </button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if isPublic}
    <div class="thread-head">Feedback received</div>
    {#if loading}
      <p class="muted">Loading feedback…</p>
    {:else if !comments.length}
      <p class="muted">No feedback yet. Share the community page link or wait for others to respond.</p>
    {:else}
      <ul class="thread">
        {#each comments as c (c.id)}
          <li class="comment" class:mine={c.mine}>
            <div class="comment-head">
              <span class="comment-author">{authorLabel(c)}</span>
              <span class="comment-meta">
                <span class="comment-date">{fmtDate(c.date)}</span>
                <CommentMenu onDelete={() => remove(c)} />
              </span>
            </div>
            <div class="comment-body">{c.body}</div>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .feedback { display: flex; flex-direction: column; gap: 1rem; }

  .share-row {
    display: flex;
    align-items: center;
    gap: 1rem;
    background: #12122a;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1rem 1.1rem;
  }
  .share-text { flex: 1; min-width: 0; }
  .share-title { font-weight: 700; font-size: 0.95rem; }
  .share-sub { font-size: 0.78rem; color: var(--muted); margin-top: 0.2rem; line-height: 1.5; }

  .share-btn {
    flex-shrink: 0;
    padding: 0.5rem 1.1rem;
    border-radius: 20px;
    border: 1px solid rgba(91,206,250,0.4);
    background: rgba(91,206,250,0.1);
    color: var(--trans-blue);
    font-size: 0.82rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s;
  }
  .share-btn.on {
    border-color: rgba(231,76,111,0.4);
    background: rgba(231,76,111,0.1);
    color: #e74c6f;
  }
  .share-btn:hover:not(:disabled) { filter: brightness(1.2); }
  .share-btn:disabled { opacity: 0.6; cursor: default; }

  .thread-head {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted);
  }
  .thread { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.6rem; }
  .comment {
    background: #12122a;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.65rem 0.85rem;
  }
  .comment.mine { border-color: rgba(91,206,250,0.35); }
  .comment-head { display: flex; justify-content: space-between; align-items: baseline; gap: 0.5rem; }
  .comment-author { font-weight: 600; font-size: 0.8rem; color: var(--trans-blue); }
  .comment-meta { display: inline-flex; align-items: center; gap: 0.4rem; }
  .comment-date { font-size: 0.68rem; color: var(--muted); }
  .comment-body { font-size: 0.875rem; margin-top: 0.3rem; line-height: 1.5; white-space: pre-wrap; }

  .muted { color: var(--muted); font-size: 0.82rem; }
  .error { color: #e74c3c; font-size: 0.8rem; }
</style>
