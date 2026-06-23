<script lang="ts">
  import { onMount } from 'svelte';
  import { listAllPublicRecordings, type PublicRecording } from '$lib/data/recordings.js';
  import { listSets, createSet, type ComparisonSet } from '$lib/data/ranking.js';
  import VoteArena from './VoteArena.svelte';
  import SetResults from './SetResults.svelte';

  type Mode = 'list' | 'create' | 'vote' | 'results';
  let mode = $state<Mode>('list');

  let sets    = $state<ComparisonSet[]>([]);
  let loading = $state(true);
  let error   = $state('');
  let active  = $state<ComparisonSet | null>(null);

  // Create-form state
  let picker       = $state<PublicRecording[]>([]);
  let pickerLoaded = $state(false);
  let chosen       = $state<Set<string>>(new Set());
  let formName     = $state('');
  let formDesc     = $state('');
  let creating     = $state(false);

  async function load() {
    loading = true; error = '';
    try { sets = await listSets(); }
    catch (e) { error = String(e); }
    finally { loading = false; }
  }
  onMount(load);

  async function openCreate() {
    mode = 'create';
    chosen = new Set(); formName = ''; formDesc = '';
    if (!pickerLoaded) {
      try { picker = await listAllPublicRecordings(); pickerLoaded = true; }
      catch (e) { error = String(e); }
    }
  }

  function toggle(id: string) {
    const next = new Set(chosen);
    next.has(id) ? next.delete(id) : next.add(id);
    chosen = next;
  }

  async function submitCreate() {
    if (chosen.size < 2 || creating) return;
    creating = true; error = '';
    try {
      await createSet(formName, formDesc, [...chosen]);
      await load();
      mode = 'list';
    } catch (e) {
      error = String(e);
    } finally {
      creating = false;
    }
  }

  function fmtDate(ts: number) {
    return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }
</script>

{#if mode === 'vote' && active}
  <div class="card"><VoteArena set={active} onBack={() => (mode = 'list')} /></div>

{:else if mode === 'results' && active}
  <div class="card"><SetResults set={active} onBack={() => (mode = 'list')} /></div>

{:else if mode === 'create'}
  <div class="card">
    <div class="card-label">New comparison set</div>
    <p class="intro">Pick 2 or more public recordings. Others will compare them pairwise.</p>

    <div class="form-row">
      <input class="inp" bind:value={formName} placeholder="Set name (e.g. Dataset A)" maxlength="120" />
    </div>
    <div class="form-row">
      <input class="inp" bind:value={formDesc} placeholder="Description (optional)" />
    </div>

    <div class="picker-head">Public recordings ({chosen.size} selected)</div>
    {#if !pickerLoaded}
      <p class="muted">Loading recordings…</p>
    {:else if !picker.length}
      <p class="muted">No public recordings yet. Mark some recordings public (Library → Feedback) first.</p>
    {:else}
      <ul class="picker">
        {#each picker as r (r.id)}
          <li class="pick-item" class:on={chosen.has(r.id)}>
            <label>
              <input type="checkbox" checked={chosen.has(r.id)} onchange={() => toggle(r.id)} />
              <span class="pick-name">{r.name}</span>
              <span class="pick-meta">
                Anon {r.authorId.slice(0, 6)} · {r.stats?.median != null ? Math.round(r.stats.median) + ' Hz' : '—'}
              </span>
            </label>
          </li>
        {/each}
      </ul>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}
    <div class="form-actions">
      <button class="ghost" onclick={() => (mode = 'list')}>Cancel</button>
      <button class="primary" disabled={chosen.size < 2 || creating} onclick={submitCreate}>
        {creating ? 'Creating…' : `Create set (${chosen.size})`}
      </button>
    </div>
  </div>

{:else}
  <div class="card">
    <div class="card-head">
      <div>
        <div class="card-label">Ranking Challenges</div>
        <p class="intro">Compare voices head-to-head and help build a femininity-scoring dataset.</p>
      </div>
      <button class="primary" onclick={openCreate}>+ New set</button>
    </div>

    {#if loading}
      <p class="muted">Loading sets…</p>
    {:else if error}
      <p class="error">{error}</p>
    {:else if !sets.length}
      <p class="muted">No comparison sets yet. Create one to start collecting rankings.</p>
    {:else}
      <ul class="sets">
        {#each sets as s (s.id)}
          <li class="set">
            <div class="set-text">
              <div class="set-name">{s.name}</div>
              {#if s.description}<div class="set-desc">{s.description}</div>{/if}
              <div class="set-meta">{s.itemCount} recordings · created {fmtDate(s.createdAt)}</div>
            </div>
            <div class="set-actions">
              <button class="primary sm" disabled={s.itemCount < 2} onclick={() => { active = s; mode = 'vote'; }}>Rank</button>
              <button class="ghost sm" onclick={() => { active = s; mode = 'results'; }}>Results</button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .card-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
  .intro { font-size: 0.82rem; color: var(--muted); margin: 0.25rem 0 0; }

  .primary {
    flex-shrink: 0; border: none; border-radius: 8px; cursor: pointer;
    background: var(--trans-pink); color: #0d0d24; font-weight: 700; font-size: 0.85rem; padding: 0.5rem 1rem;
  }
  .primary:disabled { opacity: 0.5; cursor: default; }
  .primary:hover:not(:disabled) { filter: brightness(1.08); }
  .ghost {
    border: 1px solid var(--border); border-radius: 8px; background: transparent;
    color: var(--muted); font-weight: 600; font-size: 0.85rem; padding: 0.5rem 1rem; cursor: pointer;
  }
  .ghost:hover { color: var(--text); }
  .sm { font-size: 0.78rem; padding: 0.4rem 0.85rem; }

  .sets { list-style: none; margin: 1rem 0 0; padding: 0; display: flex; flex-direction: column; gap: 0.6rem; }
  .set {
    display: flex; align-items: center; gap: 1rem;
    background: #12122a; border: 1px solid var(--border); border-radius: 10px; padding: 0.85rem 1rem;
  }
  .set-text { flex: 1; min-width: 0; }
  .set-name { font-weight: 700; font-size: 0.95rem; }
  .set-desc { font-size: 0.8rem; color: var(--muted); margin-top: 0.15rem; }
  .set-meta { font-size: 0.72rem; color: var(--muted); margin-top: 0.25rem; }
  .set-actions { display: flex; gap: 0.5rem; flex-shrink: 0; }

  .form-row { margin-top: 0.6rem; }
  .inp {
    width: 100%; background: #12122a; border: 1px solid var(--border); border-radius: 6px;
    padding: 0.5rem 0.75rem; color: var(--text); font-size: 0.875rem;
  }
  .inp:focus { outline: none; border-color: var(--trans-pink); }

  .picker-head {
    margin-top: 1rem; font-size: 0.72rem; font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.07em; color: var(--muted);
  }
  .picker { list-style: none; margin: 0.5rem 0 0; padding: 0; max-height: 320px; overflow-y: auto;
    border: 1px solid var(--border); border-radius: 8px; }
  .pick-item { border-bottom: 1px solid var(--border); }
  .pick-item:last-child { border-bottom: none; }
  .pick-item.on { background: rgba(245,169,184,0.06); }
  .pick-item label { display: flex; align-items: center; gap: 0.6rem; padding: 0.55rem 0.8rem; cursor: pointer; }
  .pick-name { font-size: 0.85rem; font-weight: 600; flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pick-meta { font-size: 0.72rem; color: var(--muted); white-space: nowrap; }

  .form-actions { display: flex; justify-content: flex-end; gap: 0.6rem; margin-top: 1rem; }

  .muted { color: var(--muted); font-size: 0.85rem; line-height: 1.6; margin-top: 1rem; }
  .error { color: #e74c3c; font-size: 0.82rem; margin-top: 0.75rem; }
</style>
