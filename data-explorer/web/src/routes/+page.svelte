<script lang="ts">
  let { data } = $props();
  const o = data.overview;

  const maxF0 = Math.max(1, ...o.f0Hist.map((h) => h.male + h.female));
  const fmt = (n: number) => n.toLocaleString();
</script>

<h2>Overview</h2>
<p class="muted">
  Pitch-crossover corpus mined from LibriSpeech: clips where pitch alone does not
  reveal gender, used to test a pitch-independent resonance (vocal-tract-length) metric.
</p>

<div class="cards">
  <div class="card"><div class="big">{fmt(o.total)}</div><div class="label">samples indexed</div></div>
  <div class="card"><div class="big">{fmt(o.kept)}</div><div class="label">crossover clips kept</div></div>
  <div class="card"><div class="big">{fmt(o.speakers)}</div><div class="label">speakers</div></div>
  {#each o.byGender.filter((g) => g.gender === 'male' || g.gender === 'female') as g}
    <div class="card"><div class="big">{fmt(g.n)}</div><div class="label">{g.gender} samples</div></div>
  {/each}
</div>

{#if o.resonance}
  <div class="panel">
    <h3 style="margin-top:0">Resonance metric (rich vocal-tract envelope)</h3>
    <p class="muted" style="margin-top:0">
      F1–F5 + VTL + LPC-cepstrum + spectral shape + formant-dynamics (VISC/movement) +
      sibilant /s/ moments — the full resonance envelope, how it moves, and the fricatives,
      not just a few formant peaks. Pitch-independent: measured on the pitch-matched set
      where F0 is uninformative. A big jump over the old single VTL number.
    </p>
    <div class="cards">
      <div class="card"><div class="big">{o.resonance.auc.toFixed(2)}</div><div class="label">AUC (pitch-matched)</div></div>
      <div class="card"><div class="big" style="color:var(--muted)">{o.resonance.vtlAuc.toFixed(2)}</div><div class="label">old VTL-only AUC</div></div>
      <div class="card"><div class="big">{(o.resonance.accuracy * 100).toFixed(0)}%</div><div class="label">accuracy</div></div>
      <div class="card"><div class="big" style="color:var(--male)">{o.resonance.vtlMale?.toFixed(1)}</div><div class="label">male VTL (cm)</div></div>
      <div class="card"><div class="big" style="color:var(--female)">{o.resonance.vtlFemale?.toFixed(1)}</div><div class="label">female VTL (cm)</div></div>
      <div class="card"><div class="big" style="color:var(--bad)">{fmt(o.resonance.hardCount)}</div><div class="label"><a href="/hard-cases">misclassified →</a></div></div>
    </div>
  </div>

  <div class="panel">
    <h3 style="margin-top:0">Combo classifier (pitch + formants)</h3>
    <p class="muted" style="margin-top:0">
      Adding F0 to the formant features lifts the full-pool classifier well above either
      alone — but barely moves the pitch-matched set, where pitch is uninformative by design.
    </p>
    <div class="cards">
      <div class="card"><div class="big">{o.resonance.comboFullAuc.toFixed(2)}</div><div class="label">combo AUC (full pool)</div></div>
      <div class="card"><div class="big" style="color:var(--muted)">{o.resonance.fullAuc.toFixed(2)}</div><div class="label">resonance-only (full pool)</div></div>
      <div class="card"><div class="big">{o.resonance.comboAuc.toFixed(2)}</div><div class="label">combo AUC (pitch-matched)</div></div>
      <div class="card"><div class="big">{(o.resonance.comboAccuracy * 100).toFixed(0)}%</div><div class="label">combo accuracy</div></div>
      <div class="card"><div class="big" style="color:var(--bad)">{fmt(o.resonance.comboHardCount)}</div><div class="label"><a href="/hard-cases?model=combo">combo misclassified →</a></div></div>
    </div>
  </div>
{:else}
  <div class="panel">
    <p class="muted">
      Resonance table not built yet. Run the Rust metric step
      (<code>pipeline metric</code>) to populate per-speaker VTL, predictions and hard cases.
    </p>
  </div>
{/if}

<div class="panel">
  <h3 style="margin-top:0">Pitch (F0) distribution by gender</h3>
  <p class="muted" style="margin-top:0">
    The overlap in the 140–180 Hz band is exactly where pitch fails and resonance must do the work.
  </p>
  <svg viewBox="0 0 520 180" width="100%" style="max-width:640px">
    {#each o.f0Hist as h, i}
      {@const x = 20 + i * 38}
      {@const mh = (h.male / maxF0) * 130}
      {@const fh = (h.female / maxF0) * 130}
      <rect x={x} y={150 - mh} width="15" height={mh} fill="var(--male)" />
      <rect x={x + 16} y={150 - fh} width="15" height={fh} fill="var(--female)" />
      {#if h.bin % 40 === 0}
        <text x={x + 8} y="168" fill="var(--muted)" font-size="9" text-anchor="middle">{h.bin}</text>
      {/if}
    {/each}
    <text x="20" y="14" fill="var(--male)" font-size="11">■ male</text>
    <text x="80" y="14" fill="var(--female)" font-size="11">■ female</text>
  </svg>
</div>
