<script lang="ts">
  import { onDestroy } from 'svelte';

  let { title, hint, accent, onBlob }: {
    title: string;
    hint: string;
    accent: 'deep' | 'bright';
    onBlob: (b: Blob | null) => void;
  } = $props();

  let status    = $state<'idle' | 'recording' | 'recorded'>('idle');
  let blobUrl   = $state<string | null>(null);
  let isPlaying = $state(false);
  let error     = $state('');
  let audioEl   = $state<HTMLAudioElement | null>(null);

  let recorder: MediaRecorder | null = null;
  let stream: MediaStream | null = null;
  let chunks: Blob[] = [];

  function stopStream() {
    stream?.getTracks().forEach((t) => t.stop());
    stream = null;
  }

  function setBlob(b: Blob | null) {
    if (blobUrl) URL.revokeObjectURL(blobUrl);
    blobUrl = b ? URL.createObjectURL(b) : null;
    onBlob(b);
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
    recorder = new MediaRecorder(stream);
    recorder.ondataavailable = (e) => { if (e.data.size) chunks.push(e.data); };
    recorder.onstop = () => {
      setBlob(new Blob(chunks, { type: recorder?.mimeType || 'audio/webm' }));
      stopStream();
    };
    recorder.start();
    status = 'recording';
  }

  function stop() {
    if (recorder && recorder.state !== 'inactive') recorder.stop();
    status = 'recorded';
  }

  function rerecord() {
    if (audioEl) { audioEl.pause(); }
    isPlaying = false;
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
