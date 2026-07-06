<script lang="ts">
  import { goto } from '$app/navigation';
  let { data } = $props();

  let mode = $state(data.mode);
  let gender = $state(data.gender);
  let model = $state(data.model);

  function apply() {
    const p = new URLSearchParams();
    if (mode !== 'all') p.set('mode', mode);
    if (gender !== 'any') p.set('gender', gender);
    if (model !== 'resonance') p.set('model', model);
    goto(`/hard-cases?${p.toString()}`);
  }

  // pick the fields for the active model
  const isCombo = $derived(data.model === 'combo');
  const pred = (r: any) => (isCombo ? r.combo_pred : r.pred);
  const prob = (r: any) => (isCombo ? r.combo_prob : r.prob_female);
  const ok = (r: any) => (isCombo ? r.combo_correct : r.correct);
</script>

<h2>Where the metric struggles</h2>
<p class="muted">
  The overlap zone — speakers where the classifier either <b>misclassifies</b> gender
  or sits <b>near its decision threshold</b> (low margin). The <b>resonance</b> model is the
  rich vocal-tract envelope (F1–F5 + VTL + LPC-cepstrum + spectral shape + formant-dynamics
  + sibilant /s/ moments, no pitch); the <b>combo</b> adds pitch level and range. On these already-ambiguous voices
  the combo helps only marginally — where pitch fails, resonance is doing the work.
</p>

{#if !data.available}
  <div class="panel">
    <p class="muted">Resonance table not built yet. Run the Rust metric step to populate hard cases.</p>
  </div>
{:else}
  <div class="controls">
    <select bind:value={model} onchange={apply}>
      <option value="resonance">resonance (rich envelope)</option>
      <option value="combo">combo (+ pitch)</option>
    </select>
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
    <span class="muted">
      {isCombo ? 'combo' : 'resonance'} matched-AUC {data.auc.toFixed(2)} · full-pool AUC
      {data.fullAuc.toFixed(2)} · {data.rows.length} shown
    </span>
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
          <td><span class="pill {pred(r)}">{pred(r)}</span></td>
          <td class="num">{r.vtl?.toFixed(1)}</td>
          <td class="num">{r.f0?.toFixed(0)}</td>
          <td class="num">{r.f1?.toFixed(0)}</td>
          <td class="num">{r.f2?.toFixed(0)}</td>
          <td class="num">{r.f3?.toFixed(0)}</td>
          <td class="num">{r.f4?.toFixed(0)}</td>
          <td class="num">{prob(r)?.toFixed(2)}</td>
          <td><span class="pill {ok(r) ? 'good' : 'bad'}">{ok(r) ? 'ok' : 'wrong'}</span></td>
          <td><audio controls preload="none" src={`/audio-speaker?speaker=${encodeURIComponent(r.speaker)}`}></audio></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
