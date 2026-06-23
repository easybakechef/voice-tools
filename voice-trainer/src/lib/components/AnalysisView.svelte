<script lang="ts">
  import WaveformCanvas from './WaveformCanvas.svelte';
  import SpectrumCanvas from './SpectrumCanvas.svelte';
  import PitchMeter     from './PitchMeter.svelte';
</script>

<div class="card">
  <div class="card-label">Waveform</div>
  <WaveformCanvas />
</div>

<div class="card">
  <div class="card-label">Frequency Spectrum</div>
  <SpectrumCanvas />
</div>

<PitchMeter />
