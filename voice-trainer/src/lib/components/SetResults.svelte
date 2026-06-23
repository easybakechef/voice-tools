<script lang="ts">
  import { onMount } from 'svelte';
  import { getSetRankings, type ComparisonSet, type RankingRow } from '$lib/data/ranking.js';

  let { set, onBack }: { set: ComparisonSet; onBack: () => void } = $props();

  let rows    = $state<RankingRow[]>([]);
  let loading = $state(true);
  let error   = $state('');

  onMount(async () => {
    try {
      rows = await getSetRankings(set.id);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  const totalVotes = $derived(rows.reduce((n, r) => n + r.comparisons, 0) / 2);
</script>

<div class="results">
  <div class="head">
    <button class="back" onclick={onBack}>← Sets</button>
    <div class="title">{set.name} — Rankings</div>
  </div>

  {#if loading}
    <p class="muted">Tallying votes…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if !rows.length}
    <p class="muted">No recordings in this set yet.</p>
  {:else}
    <p class="muted">Ranked by share of pairwise wins ("more feminine"). {Math.round(totalVotes)} comparison{totalVotes === 1 ? '' : 's'} so far.</p>
    <table>
      <thead>
        <tr><th>#</th><th>Recording</th><th>Win rate</th><th>Wins</th><th>Comparisons</th></tr>
      </thead>
      <tbody>
        {#each rows as r, i (r.recordingId)}
          <tr>
            <td class="rank">{i + 1}</td>
            <td>{r.name}</td>
            <td>
              <div class="bar-wrap">
                <div class="bar" style="width:{Math.round(r.winRate * 100)}%"></div>
                <span class="bar-label">{Math.round(r.winRate * 100)}%</span>
              </div>
            </td>
            <td class="num">{r.wins}</td>
            <td class="num">{r.comparisons}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .results { display: flex; flex-direction: column; gap: 1rem; }
  .head { display: flex; align-items: center; gap: 1rem; }
  .back {
    background: transparent; border: 1px solid var(--border); border-radius: 20px;
    color: var(--muted); font-size: 0.8rem; font-weight: 600; padding: 0.35rem 0.9rem; cursor: pointer;
  }
  .back:hover { color: var(--text); }
  .title { flex: 1; font-weight: 700; font-size: 1rem; }

  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  th {
    text-align: left; font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.06em;
    color: var(--muted); padding: 0.5rem 0.6rem; border-bottom: 1px solid var(--border);
  }
  td { padding: 0.55rem 0.6rem; border-bottom: 1px solid var(--border); }
  .rank { color: var(--muted); font-weight: 700; width: 2rem; }
  .num { text-align: right; font-variant-numeric: tabular-nums; color: var(--muted); }

  .bar-wrap { position: relative; background: #12122a; border-radius: 4px; height: 1.25rem; min-width: 120px; overflow: hidden; }
  .bar { height: 100%; background: linear-gradient(90deg, var(--trans-blue), var(--trans-pink)); border-radius: 4px; }
  .bar-label {
    position: absolute; top: 0; left: 0.4rem; line-height: 1.25rem;
    font-size: 0.72rem; font-weight: 700; color: var(--text);
  }

  .muted { color: var(--muted); font-size: 0.82rem; }
  .error { color: #e74c3c; font-size: 0.82rem; }
</style>
