<script lang="ts">
  import { onDestroy } from 'svelte';
  import { detectPitch } from '$lib/audio/wasm.js';
  import { drawSpectrum } from '$lib/audio/drawing.js';
  import { FFT_SIZE, SMOOTH } from '$lib/audio/constants.js';

  let { title, hint, accent, onBlob }: {
    title: string;
    hint: string;
    accent: 'deep' | 'bright';
    onBlob: (b: Blob | null) => void;
  } = $props();

  const BIN_GROUP = 6;

  let status    = $state<'idle' | 'recording' | 'recorded'>('idle');
  let blobUrl   = $state<string | null>(null);
  let isPlaying = $state(false);
  let error     = $state('');
  let avgHz     = $state<number | null>(null);
  let audioEl   = $state<HTMLAudioElement | null>(null);
  let canvas    = $state<HTMLCanvasElement | null>(null);

  let recorder: MediaRecorder | null = null;
  let stream: MediaStream | null = null;
  let chunks: Blob[] = [];

  // Live analysis (own AudioContext, independent of the global engine).
  let audioCtx: AudioContext | null = null;
  let analyser: AnalyserNode | null = null;
  let freqData: Float32Array<ArrayBuffer> | null = null;
  let timeData: Float32Array<ArrayBuffer> | null = null;
  let rafId = 0;
  let pitches: number[] = [];

  function stopStream() {
    stream?.getTracks().forEach((t) => t.stop());
    stream = null;
  }

  function setBlob(b: Blob | null) {
    if (blobUrl) URL.revokeObjectURL(blobUrl);
    blobUrl = b ? URL.createObjectURL(b) : null;
    onBlob(b);
  }

  function startAnalysis() {
    audioCtx = new AudioContext();
    const src = audioCtx.createMediaStreamSource(stream!);
    analyser = audioCtx.createAnalyser();
    analyser.fftSize = FFT_SIZE;
    analyser.smoothingTimeConstant = SMOOTH;
    src.connect(analyser);
    freqData = new Float32Array(analyser.frequencyBinCount);
    timeData = new Float32Array(analyser.fftSize);
    pitches = [];

    const loop = () => {
      rafId = requestAnimationFrame(loop);
      if (!analyser || !audioCtx) return;
      if (canvas) {
        analyser.getFloatFrequencyData(freqData!);
        drawSpectrum(canvas, freqData!, audioCtx.sampleRate, BIN_GROUP);
      }
      analyser.getFloatTimeDomainData(timeData!);
      const hz = detectPitch(timeData!, audioCtx.sampleRate);
      if (hz != null && hz >= 60 && hz <= 500) pitches.push(hz);
    };
    rafId = requestAnimationFrame(loop);
  }

  function stopAnalysis() {
    if (rafId) cancelAnimationFrame(rafId);
    rafId = 0;
    if (audioCtx) { void audioCtx.close(); audioCtx = null; }
    analyser = null; freqData = null; timeData = null;
  }

  async function start() {
    error = '';
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    } catch {
      error = 'Microphone access was denied.';
      return;
    }
    chunks = [];
    avgHz = null;
    recorder = new MediaRecorder(stream);
    recorder.ondataavailable = (e) => { if (e.data.size) chunks.push(e.data); };
    recorder.onstop = () => {
      setBlob(new Blob(chunks, { type: recorder?.mimeType || 'audio/webm' }));
      stopStream();
    };
    recorder.start();
    status = 'recording';
    startAnalysis();
  }

  function stop() {
    if (recorder && recorder.state !== 'inactive') recorder.stop();
    avgHz = pitches.length ? Math.round(pitches.reduce((a, b) => a + b, 0) / pitches.length) : null;
    stopAnalysis();
    status = 'recorded';
  }

  function rerecord() {
    if (audioEl) { audioEl.pause(); }
    isPlaying = false;
    avgHz = null;
    setBlob(null);
    status = 'idle';
  }

  function togglePlay() {
    if (!audioEl || !blobUrl) return;
    if (isPlaying) { audioEl.pause(); audioEl.currentTime = 0; isPlaying = false; }
    else { audioEl.play(); isPlaying = true; }
  }

  onDestroy(() => {
    if (recorder && recorder.state !== 'inactive') recorder.stop();
    stopAnalysis();
    stopStream();
    if (blobUrl) URL.revokeObjectURL(blobUrl);
  });
</script>

<div class="recorder" class:deep={accent === 'deep'} class:bright={accent === 'bright'}>
  <div class="r-title">{title}</div>
  <div class="r-hint">{hint}</div>

  <div class="r-body">
    {#if status === 'idle'}
      <button class="big rec" onclick={start}>● Record</button>
    {:else if status === 'recording'}
      <canvas bind:this={canvas} class="spectrum"></canvas>
      <button class="big stop" onclick={stop}><span class="pulse"></span>Stop</button>
    {:else}
      <div class="playback">
        <!-- svelte-ignore a11y_media_has_caption -->
        <audio bind:this={audioEl} src={blobUrl} onended={() => (isPlaying = false)}></audio>
        <button class="pb" class:on={isPlaying} onclick={togglePlay}>
          {isPlaying ? '■ Stop' : '▶ Play back'}
        </button>
        <button class="re" onclick={rerecord}>↻ Re-record</button>
      </div>
      <div class="ready">✓ Take ready</div>
    {/if}
  </div>

  <div class="avg">
    Avg pitch: <span class="avg-val">{avgHz != null ? `${avgHz} Hz` : '—'}</span>
  </div>

  {#if error}<p class="err">{error}</p>{/if}
</div>

<style>
  .recorder {
    background: #12122a;
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 1.1rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 180px;
  }
  .recorder.deep   { border-top: 3px solid #7d6cff; }
  .recorder.bright { border-top: 3px solid var(--trans-pink); }

  .r-title { font-weight: 700; font-size: 0.95rem; }
  .recorder.deep   .r-title { color: #9b8cff; }
  .recorder.bright .r-title { color: var(--trans-pink); }
  .r-hint { font-size: 0.76rem; color: var(--muted); line-height: 1.45; }

  .r-body { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.6rem; }

  .spectrum { display: block; width: 100%; height: 96px; border-radius: 8px; background: #0d0d24; }

  .avg {
    font-size: 0.78rem;
    color: var(--muted);
    text-align: center;
    border-top: 1px solid var(--border);
    padding-top: 0.5rem;
  }
  .avg-val { font-weight: 700; color: var(--text); font-variant-numeric: tabular-nums; }
  .recorder.deep   .avg-val { color: #9b8cff; }
  .recorder.bright .avg-val { color: var(--trans-pink); }

  .big {
    border: none; border-radius: 50px; cursor: pointer;
    font-size: 0.95rem; font-weight: 700; padding: 0.65rem 1.8rem;
    display: inline-flex; align-items: center; gap: 0.5rem;
  }
  .big.rec  { background: linear-gradient(135deg, var(--trans-blue), var(--trans-pink)); color: #fff; }
  .big.stop { background: linear-gradient(135deg, #e74c6f, #c0392b); color: #fff; }
  .big:hover { filter: brightness(1.08); }
  .pulse { width: 9px; height: 9px; border-radius: 50%; background: #fff; animation: blink 1s step-start infinite; }
  @keyframes blink { 50% { opacity: 0; } }

  .playback { display: flex; gap: 0.5rem; }
  .pb, .re {
    border-radius: 20px; cursor: pointer; font-size: 0.82rem; font-weight: 600; padding: 0.45rem 1rem;
  }
  .pb { border: 1px solid rgba(91,206,250,0.4); background: rgba(91,206,250,0.1); color: var(--trans-blue); }
  .pb.on { border-color: rgba(231,76,111,0.4); background: rgba(231,76,111,0.1); color: #e74c6f; }
  .re { border: 1px solid var(--border); background: transparent; color: var(--muted); }
  .re:hover { color: var(--text); }
  .ready { font-size: 0.78rem; color: var(--trans-blue); font-weight: 600; }

  .err { color: #e74c3c; font-size: 0.78rem; text-align: center; }
</style>
