<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listMyPairs, deletePair, getSampleUrl, type DatasetPair } from '$lib/data/dataset.js';

  let pairs   = $state<DatasetPair[]>([]);
  let loading = $state(true);
  let error   = $state('');
  let playing = $state<string | null>(null); // `${pairId}:${label}`
  let busyId  = $state<string | null>(null);

  let audio: HTMLAudioElement | null = null;

  async function load() {
    loading = true; error = '';
    try { pairs = await listMyPairs(); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }
  onMount(load);

  function stopAudio() {
    if (audio) { audio.pause(); audio = null; }
    playing = null;
  }

  async function play(pairId: string, label: 'deep' | 'bright', path: string | null) {
    if (!path) return;
    const key = `${pairId}:${label}`;
    if (playing === key) { stopAudio(); return; }
    stopAudio();
    const url = await getSampleUrl(path);
    audio = new Audio(url);
    audio.onended = () => { playing = null; audio = null; };
    playing = key;
    await audio.play();
  }

  async function remove(pair: DatasetPair) {
    if (busyId) return;
    busyId = pair.id;
    stopAudio();
    try {
      await deletePair(pair.id);
      pairs = pairs.filter((p) => p.id !== pair.id);
    } catch (e) {
      error = String(e);
    } finally {
      busyId = null;
    }
  }

  function fmtDate(ts: number) {
    return new Date(ts).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }

  onDestroy(stopAudio);
</script>

{#if loading}
  <p class="muted">Loading your recordings…</p>
{:else if error}
  <p class="error">{error}</p>
{:else if !pairs.length}
  <p class="muted">You haven't submitted any pairs yet. Record one above to get started.</p>
{:else}
  <ul class="pairs">
    {#each pairs as p (p.id)}
      <li class="pair">
        <div class="pair-main">
          <div class="phrase">“{p.phrase}”</div>
          <div class="meta">{fmtDate(p.createdAt)} · <span class="uuid">{p.id.slice(0, 8)}</span></div>
          <div class="takes">
            <button class="take deep" class:on={playing === `${p.id}:deep`} disabled={!p.deepPath} onclick={() => play(p.id, 'deep', p.deepPath)}>
              {playing === `${p.id}:deep` ? '■' : '▶'} Deep
            </button>
            <button class="take bright" class:on={playing === `${p.id}:bright`} disabled={!p.brightPath} onclick={() => play(p.id, 'bright', p.brightPath)}>
              {playing === `${p.id}:bright` ? '■' : '▶'} Bright
            </button>
          </div>
        </div>
        <button class="del" disabled={busyId === p.id} title="Delete pair" onclick={() => remove(p)}>
          {busyId === p.id ? '…' : '🗑'}
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .pairs { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.6rem; }
  .pair {
    display: flex; align-items: center; gap: 1rem;
    background: #12122a; border: 1px solid var(--border); border-radius: 10px; padding: 0.85rem 1rem;
  }
  .pair-main { flex: 1; min-width: 0; }
  .phrase { font-size: 0.9rem; font-weight: 600; }
  .meta { font-size: 0.72rem; color: var(--muted); margin-top: 0.2rem; }
  .uuid { font-family: ui-monospace, monospace; }

  .takes { display: flex; gap: 0.5rem; margin-top: 0.5rem; }
  .take {
    border-radius: 20px; cursor: pointer; font-size: 0.78rem; font-weight: 700; padding: 0.35rem 0.9rem;
  }
  .take.deep   { border: 1px solid rgba(125,108,255,0.4); background: rgba(125,108,255,0.12); color: #9b8cff; }
  .take.bright { border: 1px solid rgba(245,169,184,0.4); background: rgba(245,169,184,0.12); color: var(--trans-pink); }
  .take.on { filter: brightness(1.3); box-shadow: 0 0 8px currentColor; }
  .take:disabled { opacity: 0.4; cursor: default; }

  .del {
    flex-shrink: 0; background: transparent; border: 1px solid var(--border); border-radius: 8px;
    color: var(--muted); cursor: pointer; font-size: 0.9rem; padding: 0.4rem 0.6rem;
  }
  .del:hover:not(:disabled) { color: #e74c3c; border-color: rgba(231,76,60,0.4); }
  .del:disabled { opacity: 0.5; cursor: default; }

  .muted { color: var(--muted); font-size: 0.85rem; line-height: 1.6; }
  .error { color: #e74c3c; font-size: 0.85rem; }
</style>
