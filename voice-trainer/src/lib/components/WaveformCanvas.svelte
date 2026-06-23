<script lang="ts">
  import { engine } from '$lib/audio/engine.svelte.js';
  import { drawWaveform, clearCanvas } from '$lib/audio/drawing.js';

  let canvas = $state<HTMLCanvasElement | null>(null);

  $effect(() => {
    const active = engine.activeType;
    if (!canvas) return;
    if (!active) { clearCanvas(canvas); return; }

    let animId: number;
    const draw = () => {
      animId = requestAnimationFrame(draw);
      drawWaveform(canvas!, engine.timeDomainData);
    };
    animId = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(animId);
  });
</script>

<canvas bind:this={canvas}></canvas>

<style>
  canvas {
    display: block;
    width: 100%;
    height: 100px;
    border-radius: 8px;
    background: #12122a;
  }
</style>
