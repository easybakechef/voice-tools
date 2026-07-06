<script lang="ts">
  import { goto } from '$app/navigation';
  let { data } = $props();

  let q = $state(data.query.q ?? '');
  let gender = $state(data.query.gender ?? 'any');
  let dataset = $state(data.query.dataset ?? 'any');
  let keptOnly = $state(data.query.keptOnly ?? false);
  let f0min = $state(data.query.f0min ?? '');
  let f0max = $state(data.query.f0max ?? '');

  function apply() {
    const p = new URLSearchParams();
    if (q) p.set('q', q);
    if (gender !== 'any') p.set('gender', gender);
    if (dataset !== 'any') p.set('dataset', dataset);
    if (keptOnly) p.set('kept', '1');
    if (f0min !== '') p.set('f0min', String(f0min));
    if (f0max !== '') p.set('f0max', String(f0max));
    goto(`/explorer?${p.toString()}`, { keepFocus: true });
  }
  const base = (path: string) => path.split('/').pop();
</script>

<h2>Sample explorer</h2>
<div class="controls">
  <input placeholder="search speaker / path" bind:value={q} onkeydown={(e) => e.key === 'Enter' && apply()} />
  <select bind:value={gender}>
    <option value="any">any gender</option>
    <option value="male">male</option>
    <option value="female">female</option>
  </select>
  <select bind:value={dataset}>
    <option value="any">any dataset</option>
    {#each data.datasetList as d}<option value={d}>{d}</option>{/each}
  </select>
  <input type="number" placeholder="F0 min" style="width:90px" bind:value={f0min} />
  <input type="number" placeholder="F0 max" style="width:90px" bind:value={f0max} />
  <label class="muted"><input type="checkbox" bind:checked={keptOnly} /> kept only</label>
  <button onclick={apply} style="cursor:pointer">Apply</button>
</div>

<p class="muted">{data.total.toLocaleString()} matching samples — showing {data.rows.length}</p>

<table>
  <thead>
    <tr>
      <th>speaker</th><th>dataset</th><th>gender</th><th>clip</th>
      <th class="num">F0</th><th class="num">F1</th><th class="num">F2</th>
      <th class="num">voiced</th><th>kept</th><th>audio</th>
    </tr>
  </thead>
  <tbody>
    {#each data.rows as r}
      <tr>
        <td><a href={`/speaker/${r.speaker}`}>{r.speaker}</a></td>
        <td>{r.dataset}</td>
        <td>{#if r.gender}<span class="pill {r.gender}">{r.gender}</span>{/if}</td>
        <td class="muted">{base(r.path)}</td>
        <td class="num">{r.f0_median?.toFixed(0) ?? '–'}</td>
        <td class="num">{r.f1_median?.toFixed(0) ?? '–'}</td>
        <td class="num">{r.f2_median?.toFixed(0) ?? '–'}</td>
        <td class="num">{r.voiced_frac != null ? (r.voiced_frac * 100).toFixed(0) + '%' : '–'}</td>
        <td>{r.kept ? '✓' : ''}</td>
        <td>
          {#if r.kept}
            <audio controls preload="none" src={`/audio?path=${encodeURIComponent(r.path)}`}></audio>
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
