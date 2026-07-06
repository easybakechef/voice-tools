<script lang="ts">
  let { data } = $props();
  const r: any = data.resonance;
  const base = (p: string) => p.split('/').pop();
</script>

<p><a href="/explorer">← explorer</a></p>
<h2>{data.speaker}</h2>

{#if r}
  <div class="panel">
    <h3 style="margin-top:0">Resonance profile</h3>
    <div class="cards">
      <div class="card"><div class="big">{r.vtl?.toFixed(1)}</div><div class="label">VTL (cm)</div></div>
      <div class="card"><div class="big"><span class="pill {r.gender}">{r.gender}</span></div><div class="label">actual</div></div>
      <div class="card"><div class="big"><span class="pill {r.pred}">{r.pred}</span></div><div class="label">predicted</div></div>
      <div class="card">
        <div class="big"><span class="pill {r.correct ? 'good' : 'bad'}">{r.correct ? 'correct' : 'wrong'}</span></div>
        <div class="label">margin {r.margin?.toFixed(2)}</div>
      </div>
    </div>
    <p class="muted" style="margin-bottom:0">
      F0 {r.f0?.toFixed(0)} · F1 {r.f1?.toFixed(0)} · F2 {r.f2?.toFixed(0)} ·
      F3 {r.f3?.toFixed(0)} · F4 {r.f4?.toFixed(0)} Hz
      {#if r.in_matched}· in pitch-matched set{/if}
    </p>
  </div>
{/if}

<div class="panel">
  <h3 style="margin-top:0">Clips ({data.clips.length})</h3>
  <table>
    <thead>
      <tr><th>clip</th><th>dataset</th><th class="num">dur</th><th class="num">F0</th><th class="num">F1</th><th class="num">F2</th><th>kept</th><th>audio</th></tr>
    </thead>
    <tbody>
      {#each data.clips as c}
        <tr>
          <td class="muted">{base(c.path)}</td>
          <td>{c.dataset}</td>
          <td class="num">{c.duration?.toFixed(1) ?? '–'}</td>
          <td class="num">{c.f0_median?.toFixed(0) ?? '–'}</td>
          <td class="num">{c.f1_median?.toFixed(0) ?? '–'}</td>
          <td class="num">{c.f2_median?.toFixed(0) ?? '–'}</td>
          <td>{c.kept ? '✓' : ''}</td>
          <td>{#if c.kept}<audio controls preload="none" src={`/audio?path=${encodeURIComponent(c.path)}`}></audio>{/if}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
