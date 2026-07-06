<script lang="ts">
  import { goto } from '$app/navigation';
  let { data } = $props();

  let mode = $state(data.mode);
  let gender = $state(data.gender);
  function apply() {
    const p = new URLSearchParams();
    if (mode !== 'all') p.set('mode', mode);
    if (gender !== 'any') p.set('gender', gender);
    goto(`/hard-cases?${p.toString()}`);
  }
</script>

<h2>Where the metric struggles</h2>
<p class="muted">
  These are the speakers in the overlap zone — where the vocal-tract-length metric
  either <b>misclassifies</b> gender or is <b>near its decision threshold</b> (low margin).
  This is the real ambiguity that pitch also can't resolve: short-tract men and
  long-tract women whose resonance sits on the wrong side.
</p>

{#if !data.available}
  <div class="panel">
    <p class="muted">
      Resonance table not built yet. Run the Rust metric step to populate hard cases.
    </p>
  </div>
{:else}
  <div class="controls">
    <select bind:value={mode} onchange={apply}>
      <option value="all">misclassified or ambiguous</option>
      <option value="wrong">misclassified only</option>
      <option value="ambiguous">ambiguous (low margin)</option>
    </select>
    <select bind:value={gender} onchange={apply}>
      <option value="any">any gender</option>
      <option value="male">male</option>
      <option value="female">female</option>
    </select>
    <span class="muted">AUC {data.auc.toFixed(2)} · VTL threshold ≈ {data.threshold.toFixed(1)} cm · {data.rows.length} shown</span>
  </div>

  <table>
    <thead>
      <tr>
        <th>speaker</th><th>actual</th><th>predicted</th>
        <th class="num">VTL</th><th class="num">F0</th>
        <th class="num">F1</th><th class="num">F2</th><th class="num">F3</th><th class="num">F4</th>
        <th class="num">P(female)</th><th>result</th><th>audio</th>
      </tr>
    </thead>
    <tbody>
      {#each data.rows as r}
        <tr>
          <td><a href={`/speaker/${r.speaker}`}>{r.speaker}</a></td>
          <td><span class="pill {r.gender}">{r.gender}</span></td>
          <td><span class="pill {r.pred}">{r.pred}</span></td>
          <td class="num">{r.vtl?.toFixed(1)}</td>
          <td class="num">{r.f0?.toFixed(0)}</td>
          <td class="num">{r.f1?.toFixed(0)}</td>
          <td class="num">{r.f2?.toFixed(0)}</td>
          <td class="num">{r.f3?.toFixed(0)}</td>
          <td class="num">{r.f4?.toFixed(0)}</td>
          <td class="num">{r.prob_female?.toFixed(2)}</td>
          <td><span class="pill {r.correct ? 'good' : 'bad'}">{r.correct ? 'ok' : 'wrong'}</span></td>
          <td><audio controls preload="none" src={`/audio-speaker?speaker=${encodeURIComponent(r.speaker)}`}></audio></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
