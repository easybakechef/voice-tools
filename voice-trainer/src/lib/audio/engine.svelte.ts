import { FFT_SIZE, SMOOTH, TARGET_LO, TARGET_HI, CLIPS, CLIP_DUR } from './constants.js';
import type { ActiveType, PitchPoint } from './types.js';
import { detectPitch, tryLoadWasm } from './wasm.js';

class AudioEngine {
  // ── Reactive UI state ────────────────────────────────────────────────────
  activeType    = $state<ActiveType>(null);
  activePreset  = $state<string | null>(null);
  smoothPitch   = $state<number | null>(null);
  pitchHint     = $state('');
  pitchInTarget = $state(false);
  monitorEnabled = $state(false);
  binGroup      = $state(1);
  snapshotReady = $state(false);
  statusMsg     = $state('Click to begin — microphone access required.');
  isRecording        = $state(false);
  isRecordingPaused  = $state(false);
  playbackProgress   = $state(0);
  playbackTime       = $state('0:00 / 0:00');
  recordingBlob      = $state<Blob | null>(null);

  // ── Selected clip for snapshot display (Playback tab) ───────────────────
  selectedPreset       = $state<string | null>(null);
  clipSnapshotsVersion = $state(0);
  clipAnalysisState    = $state<Partial<Record<string, 'loading' | 'done' | 'error'>>>({});
  clipSnapshots: Partial<Record<string, { pitchLog: PitchPoint[]; specFrameStore: Float32Array[] }>> = {};

  // ── Snapshot data (non-reactive — only read when snapshotReady fires) ────
  pitchLog: PitchPoint[] = [];
  specFrameStore: Float32Array[] = [];

  // ── High-frequency canvas data (non-reactive, read each RAF tick) ────────
  frequencyData  = new Float32Array(FFT_SIZE / 2);
  timeDomainData = new Float32Array(FFT_SIZE);

  // ── Private audio graph ──────────────────────────────────────────────────
  #ctx: AudioContext | null = null;
  #analyser: AnalyserNode | null = null;
  #monitorGain: GainNode | null = null;
  #activeStream: MediaStream | null = null;
  #audioEl: HTMLAudioElement | null = null;
  #mediaRecorder: MediaRecorder | null = null;
  #recChunks: Blob[] = [];
  #animId: number | null = null;
  #recStartTime   = 0;
  #lastLogTime    = 0;
  #frameCount     = 0;
  #pauseStartTime = 0;
  #totalPausedMs  = 0;

  get sampleRate(): number { return this.#ctx?.sampleRate ?? 44100; }

  constructor() {
    tryLoadWasm();
  }

  #ensureCtx(): AudioContext {
    if (!this.#ctx || this.#ctx.state === 'closed') {
      this.#ctx = new AudioContext();
      this.#analyser = this.#ctx.createAnalyser();
      this.#analyser.fftSize = FFT_SIZE;
      this.#analyser.smoothingTimeConstant = SMOOTH;
      this.#monitorGain = this.#ctx.createGain();
      this.#monitorGain.gain.value = 0;
      this.#analyser.connect(this.#monitorGain);
      this.#monitorGain.connect(this.#ctx.destination);
    }
    if (this.#ctx.state === 'suspended') this.#ctx.resume();
    return this.#ctx;
  }

  #teardown(): void {
    if (this.#mediaRecorder && this.#mediaRecorder.state !== 'inactive') {
      this.#mediaRecorder.stop();
    }
    this.#mediaRecorder = null;
    if (this.#activeStream) {
      this.#activeStream.getTracks().forEach(t => t.stop());
      this.#activeStream = null;
    }
    if (this.#audioEl) {
      try { this.#audioEl.pause(); this.#audioEl.src = ''; } catch { /* ignore */ }
      this.#audioEl = null;
    }
    if (this.#animId !== null) {
      cancelAnimationFrame(this.#animId);
      this.#animId = null;
    }
    if (this.#monitorGain) this.#monitorGain.gain.value = 0;
    this.activeType         = null;
    this.activePreset       = null;
    this.smoothPitch        = null;
    this.pitchHint          = '';
    this.pitchInTarget      = false;
    this.isRecording        = false;
    this.isRecordingPaused  = false;
    this.playbackProgress   = 0;
  }

  stopAll(): void {
    const hadData  = this.pitchLog.length > 0;
    const wasClip  = this.activeType === 'clip';
    const clipKey  = this.activePreset;
    this.#teardown();
    if (hadData) {
      if (wasClip && clipKey) {
        this.clipSnapshots[clipKey] = {
          pitchLog:      [...this.pitchLog],
          specFrameStore: [...this.specFrameStore],
        };
        this.clipSnapshotsVersion++;
      }
      this.snapshotReady = true;
    }
  }

  // Pause a live recording for review without losing the session.
  async pauseForReview(): Promise<void> {
    if (this.activeType !== 'live' || !this.#mediaRecorder) return;
    if (this.#mediaRecorder.state !== 'recording') return;

    // Wait for the current buffered chunk before pausing
    await new Promise<void>(resolve => {
      this.#mediaRecorder!.addEventListener('dataavailable', () => resolve(), { once: true });
      this.#mediaRecorder!.requestData();
    });

    this.#mediaRecorder.pause();
    this.#pauseStartTime = performance.now();

    if (this.#animId !== null) {
      cancelAnimationFrame(this.#animId);
      this.#animId = null;
    }

    this.isRecording       = false;
    this.isRecordingPaused = true;
    this.statusMsg         = 'Paused — review your recording or keep going.';
    // Temp blob so the playback widget can work immediately
    this.recordingBlob     = new Blob(this.#recChunks, { type: 'audio/webm' });
    if (this.pitchLog.length > 0) this.snapshotReady = true;
  }

  resumeRecording(): void {
    if (this.activeType !== 'live' || !this.#mediaRecorder) return;
    if (this.#mediaRecorder.state !== 'paused') return;

    this.#totalPausedMs   += performance.now() - this.#pauseStartTime;
    this.recordingBlob     = null;
    this.snapshotReady     = false;
    this.isRecordingPaused = false;
    this.isRecording       = true;
    this.statusMsg         = 'Recording…';
    this.#mediaRecorder.resume();
    this.#startLoop();
  }

  stopRecording(): void {
    if (this.activeType !== 'live') return;
    const hadData = this.pitchLog.length > 0;
    this.#teardown(); // calls MediaRecorder.stop() → onstop fires → recordingBlob set to final blob
    this.statusMsg = 'Recording finished.';
    if (hadData) this.snapshotReady = true;
  }

  // Select a clip for snapshot display without starting playback.
  selectPreset(key: string): void {
    this.snapshotReady  = false;
    this.selectedPreset = key;
    if (!this.clipSnapshots[key] && this.clipAnalysisState[key] !== 'loading') {
      void this.#analyzeClipOffline(key);
    }
  }

  async #analyzeClipOffline(key: string): Promise<void> {
    const clip = CLIPS[key as keyof typeof CLIPS];
    if (!clip) return;

    this.clipAnalysisState[key] = 'loading';
    try {
      const res = await fetch(clip.url, { mode: 'cors' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const arrayBuffer = await res.arrayBuffer();

      // Decode in a temporary context (decodeAudioData works regardless of state)
      const tmpCtx = new AudioContext();
      const audioBuffer = await tmpCtx.decodeAudioData(arrayBuffer);
      tmpCtx.close();

      const sr          = audioBuffer.sampleRate;
      const duration    = Math.min(CLIP_DUR, audioBuffer.duration);
      const numSamples  = Math.ceil(duration * sr);
      const blockSize   = 4096;
      const logInterval = Math.max(1, Math.round(0.08 * sr / blockSize)); // ~80 ms between pitch samples

      const offlineCtx = new OfflineAudioContext(1, numSamples, sr);

      const source = offlineCtx.createBufferSource();
      source.buffer = audioBuffer;
      source.start(0);

      const analyser = offlineCtx.createAnalyser();
      analyser.fftSize              = FFT_SIZE;
      analyser.smoothingTimeConstant = SMOOTH;

      const freqData      = new Float32Array(analyser.frequencyBinCount);
      const timeData      = new Float32Array(FFT_SIZE);
      const specFrameStore: Float32Array[] = [];
      const pitchLog:       PitchPoint[]   = [];
      let blockIdx = 0;

      // ScriptProcessorNode fires onaudioprocess during startRendering,
      // giving us per-block access to the analyser data.
      const proc = offlineCtx.createScriptProcessor(blockSize, 1, 1);
      proc.onaudioprocess = (e: AudioProcessingEvent) => {
        analyser.getFloatFrequencyData(freqData);
        analyser.getFloatTimeDomainData(timeData);

        if (blockIdx % 4 === 0) {
          const snap = new Float32Array(freqData.length);
          for (let i = 0; i < freqData.length; i++) snap[i] = Math.max(-100, freqData[i]);
          specFrameStore.push(snap);
        }

        if (blockIdx % logInterval === 0) {
          const hz = detectPitch(timeData, sr);
          const t  = (blockIdx * blockSize) / sr;
          if (hz !== null) pitchLog.push({ t, hz });
        }

        // Must pass audio through for the graph to render
        e.outputBuffer.getChannelData(0).set(e.inputBuffer.getChannelData(0));
        blockIdx++;
      };

      source.connect(analyser);
      analyser.connect(proc);
      proc.connect(offlineCtx.destination);

      await offlineCtx.startRendering();

      if (pitchLog.length === 0 && specFrameStore.length === 0) {
        throw new Error('No data collected — ScriptProcessorNode may not fire in this browser');
      }

      // Don't overwrite data from a real-time play session
      if (!this.clipSnapshots[key]) {
        this.clipSnapshots[key] = { pitchLog, specFrameStore };
        this.clipSnapshotsVersion++;
      }
      this.clipAnalysisState[key] = 'done';
    } catch (err) {
      console.error('[VoiceTrainer] Offline analysis failed for', key, err);
      this.clipAnalysisState[key] = 'error';
    }
  }

  async startRecording(): Promise<void> {
    if (this.activeType === 'live') {
      const hadData = this.pitchLog.length > 0;
      this.#teardown();
      this.statusMsg = 'Recording stopped.';
      if (hadData) this.snapshotReady = true;
      return;
    }

    this.#teardown();
    this.snapshotReady = false;
    this.recordingBlob = null;
    this.pitchLog = [];
    this.specFrameStore = [];
    this.#recChunks = [];
    this.#frameCount = 0;

    try {
      const ctx = this.#ensureCtx();
      this.#activeStream = await navigator.mediaDevices.getUserMedia({ audio: true });
      ctx.createMediaStreamSource(this.#activeStream).connect(this.#analyser!);
      this.#monitorGain!.gain.value = this.monitorEnabled ? 1 : 0;

      this.#mediaRecorder = new MediaRecorder(this.#activeStream);
      this.#mediaRecorder.ondataavailable = (e) => {
        if (e.data.size > 0) this.#recChunks.push(e.data);
      };
      this.#mediaRecorder.onstop = () => {
        this.recordingBlob = new Blob(this.#recChunks, { type: 'audio/webm' });
      };
      this.#mediaRecorder.start();

      this.#recStartTime  = performance.now();
      this.#lastLogTime   = 0;
      this.#totalPausedMs = 0;
      this.activeType     = 'live';
      this.isRecording    = true;
      this.statusMsg     = 'Recording…';
      this.#startLoop();
    } catch {
      this.statusMsg = 'Microphone access denied.';
    }
  }

  async startClip(key: string): Promise<void> {
    if (this.activeType === 'clip' && this.activePreset === key) {
      this.stopAll();
      return;
    }
    this.#teardown();
    this.snapshotReady  = false;
    this.selectedPreset = key;
    this.pitchLog = [];
    this.specFrameStore = [];
    this.#frameCount = 0;

    const clip = CLIPS[key as keyof typeof CLIPS];
    if (!clip) return;

    const ctx = this.#ensureCtx();
    this.#audioEl = new Audio();
    this.#audioEl.crossOrigin = 'anonymous';
    this.#audioEl.src = clip.url;

    const mediaSrc = ctx.createMediaElementSource(this.#audioEl);
    mediaSrc.connect(this.#analyser!);

    this.#audioEl.addEventListener('timeupdate', () => {
      if (this.#audioEl && this.#audioEl.currentTime >= CLIP_DUR) this.stopAll();
    });
    this.#audioEl.addEventListener('ended', () => this.stopAll());
    this.#audioEl.addEventListener('error', () => {
      this.statusMsg = 'Failed to load clip.';
      this.#teardown();
    });

    await this.#audioEl.play();
    this.#monitorGain!.gain.value = 1;
    this.activeType    = 'clip';
    this.activePreset  = key;
    this.#recStartTime = performance.now();
    this.#lastLogTime  = 0;
    this.#startLoop();
  }

  async loadFile(file: File): Promise<void> {
    this.#teardown();
    this.snapshotReady = false;
    this.pitchLog = [];
    this.specFrameStore = [];
    this.#frameCount = 0;

    const ctx = this.#ensureCtx();
    const url = URL.createObjectURL(file);
    this.#audioEl = new Audio();
    this.#audioEl.crossOrigin = 'anonymous';
    this.#audioEl.src = url;

    const mediaSrc = ctx.createMediaElementSource(this.#audioEl);
    mediaSrc.connect(this.#analyser!);

    this.#audioEl.addEventListener('ended', () => {
      URL.revokeObjectURL(url);
      this.stopAll();
    });

    await this.#audioEl.play();
    this.#monitorGain!.gain.value = 1;
    this.activeType    = 'file';
    this.#recStartTime = performance.now();
    this.#lastLogTime  = 0;
    this.#startLoop();
  }

  async loadBlob(blob: Blob, name: string): Promise<void> {
    const url = URL.createObjectURL(blob);
    await this.#loadAudioUrl(url, name, () => URL.revokeObjectURL(url));
  }

  async loadUrl(url: string, name: string): Promise<void> {
    await this.#loadAudioUrl(url, name);
  }

  async #loadAudioUrl(url: string, name: string, onEnd?: () => void): Promise<void> {
    this.#teardown();
    this.snapshotReady = false;
    this.recordingBlob = null;
    this.pitchLog = [];
    this.specFrameStore = [];
    this.#frameCount = 0;

    const ctx = this.#ensureCtx();
    // crossOrigin must be set BEFORE src so the fetch is a CORS request. The URL
    // is now a cross-origin Supabase signed URL (:54321); without this,
    // createMediaElementSource taints the stream and outputs silence.
    this.#audioEl = new Audio();
    this.#audioEl.crossOrigin = 'anonymous';
    this.#audioEl.src = url;

    const mediaSrc = ctx.createMediaElementSource(this.#audioEl);
    mediaSrc.connect(this.#analyser!);

    this.#audioEl.addEventListener('ended', () => {
      onEnd?.();
      this.stopAll();
    });

    await this.#audioEl.play();
    this.#monitorGain!.gain.value = 1;
    this.activeType    = 'file';
    this.activePreset  = name;
    this.#recStartTime = performance.now();
    this.#lastLogTime  = 0;
    this.#startLoop();
  }

  toggleMonitor(): void {
    this.monitorEnabled = !this.monitorEnabled;
    if (this.#monitorGain && this.activeType === 'live') {
      this.#monitorGain.gain.value = this.monitorEnabled ? 1 : 0;
    }
  }

  #startLoop(): void {
    if (this.#animId !== null) cancelAnimationFrame(this.#animId);
    const tick = () => {
      if (!this.activeType || !this.#analyser) return;
      this.#animId = requestAnimationFrame(tick);
      this.#analyser.getFloatFrequencyData(this.frequencyData);
      this.#analyser.getFloatTimeDomainData(this.timeDomainData);
      this.#updatePitch();
      this.#updatePlaybackProgress();
      this.#collectSnapshot();
    };
    this.#animId = requestAnimationFrame(tick);
  }

  #updatePitch(): void {
    const raw = detectPitch(this.timeDomainData, this.sampleRate);
    if (raw === null) {
      this.smoothPitch   = null;
      this.pitchHint     = '';
      this.pitchInTarget = false;
      return;
    }
    this.smoothPitch = this.smoothPitch === null ? raw : 0.3 * raw + 0.7 * this.smoothPitch;
    const hz = Math.round(this.smoothPitch);
    if (hz < TARGET_LO) {
      this.pitchHint     = `▲ Raise pitch ~${TARGET_LO - hz} Hz`;
      this.pitchInTarget = false;
    } else {
      this.pitchHint     = 'In target range ✓';
      this.pitchInTarget = true;
    }
  }

  #updatePlaybackProgress(): void {
    if (!this.#audioEl) return;
    const cur = this.#audioEl.currentTime;
    const dur = this.activeType === 'clip'
      ? Math.min(CLIP_DUR, this.#audioEl.duration || CLIP_DUR)
      : (this.#audioEl.duration || 0);
    if (!dur) return;
    this.playbackProgress = cur / dur;
    const fmt = (s: number) =>
      `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
    this.playbackTime = `${fmt(cur)} / ${fmt(dur)}`;
  }

  #collectSnapshot(): void {
    this.#frameCount++;
    if (this.#frameCount % 4 === 0) {
      const snap = new Float32Array(this.frequencyData.length);
      for (let i = 0; i < this.frequencyData.length; i++) {
        snap[i] = Math.max(-100, this.frequencyData[i]);
      }
      this.specFrameStore.push(snap);
    }
    const now = performance.now();
    if (this.smoothPitch !== null && now - this.#lastLogTime >= 80) {
      this.#lastLogTime = now;
      this.pitchLog.push({ t: (now - this.#recStartTime - this.#totalPausedMs) / 1000, hz: this.smoothPitch });
    }
  }
}

export const engine = new AudioEngine();
