import {
  BAR_MIN, BAR_MAX, MIN_HZ, MAX_HZ, TARGET_LO, TARGET_HI, FORMANT_BANDS,
} from './constants.js';
import type { PitchPoint, SnapshotStats } from './types.js';

// ── Canvas helpers ───────────────────────────────────────────────────────────

export function resizeDPR(canvas: HTMLCanvasElement): boolean {
  const dpr = window.devicePixelRatio || 1;
  const w = canvas.clientWidth, h = canvas.clientHeight;
  if (w === 0 || h === 0) return false;
  if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.getContext('2d')!.scale(dpr, dpr);
  }
  return true;
}

export function clearCanvas(canvas: HTMLCanvasElement): void {
  const ctx = canvas.getContext('2d');
  if (ctx) ctx.clearRect(0, 0, canvas.width, canvas.height);
}

// ── Log-scale frequency → x coordinate ─────────────────────────────────────

function freqToX(hz: number, W: number): number {
  const lo = Math.log(BAR_MIN), hi = Math.log(BAR_MAX);
  return ((Math.log(Math.max(BAR_MIN, hz)) - lo) / (hi - lo)) * W;
}

function hzToPct(hz: number): number {
  const lo = Math.log(MIN_HZ), hi = Math.log(MAX_HZ);
  return ((Math.log(Math.max(MIN_HZ, Math.min(MAX_HZ, hz))) - lo) / (hi - lo)) * 100;
}

function hzToY(hz: number, H: number): number {
  const lo = Math.log(MIN_HZ), hi = Math.log(MAX_HZ);
  return H * (1 - (Math.log(Math.max(MIN_HZ, Math.min(MAX_HZ, hz))) - lo) / (hi - lo));
}

// ── Live waveform ────────────────────────────────────────────────────────────

export function drawWaveform(canvas: HTMLCanvasElement, data: Float32Array): void {
  if (!resizeDPR(canvas)) return;
  const ctx = canvas.getContext('2d')!;
  const W = canvas.clientWidth, H = canvas.clientHeight;
  ctx.clearRect(0, 0, W, H);
  ctx.strokeStyle = '#F5A9B8';
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  const step = data.length / W;
  for (let x = 0; x < W; x++) {
    const y = (1 - data[Math.floor(x * step)]) * H / 2;
    x === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
  }
  ctx.stroke();
}

// ── Live spectrum ────────────────────────────────────────────────────────────

export function drawSpectrum(
  canvas: HTMLCanvasElement,
  data: Float32Array,
  sampleRate: number,
  binGroup: number
): void {
  if (!resizeDPR(canvas)) return;
  const ctx = canvas.getContext('2d')!;
  const W = canvas.clientWidth, H = canvas.clientHeight;
  const hzPerBin = sampleRate / (2 * data.length);

  ctx.clearRect(0, 0, W, H);

  FORMANT_BANDS.forEach(f => {
    ctx.fillStyle = f.color + '1e';
    ctx.fillRect(freqToX(f.lo, W), 0, freqToX(f.hi, W) - freqToX(f.lo, W), H);
    ctx.fillStyle = f.color + 'bb';
    ctx.font = '11px system-ui';
    ctx.fillText(f.label, freqToX(f.lo, W) + 4, 14);
  });

  for (let i = 1; i < data.length; i += binGroup) {
    const hz1 = i * hzPerBin, hz2 = (i + binGroup) * hzPerBin;
    if (hz2 < BAR_MIN || hz1 > BAR_MAX) continue;
    let sum = 0, count = 0;
    for (let j = i; j < i + binGroup && j < data.length; j++) { sum += data[j]; count++; }
    const barH = Math.max(0, ((sum / count) + 100) / 100) * H;
    const x1 = freqToX(Math.max(hz1, BAR_MIN), W);
    const bW = Math.max(1, freqToX(Math.min(hz2, BAR_MAX), W) - x1);
    const cHz = (hz1 + hz2) / 2;
    let color = '#4a4a6a';
    for (const f of FORMANT_BANDS) { if (cHz >= f.lo && cHz <= f.hi) { color = f.color; break; } }
    ctx.fillStyle = color;
    ctx.fillRect(x1, H - barH, bW, barH);
  }

  [100, 200, 300, 500, 800, 1000, 1500, 2000, 3000, 4000].forEach(hz => {
    const x = freqToX(hz, W);
    ctx.strokeStyle = '#2a2a4a'; ctx.lineWidth = 1;
    ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, H); ctx.stroke();
    ctx.fillStyle = '#6b6b8a'; ctx.font = '10px system-ui';
    ctx.fillText(hz >= 1000 ? hz / 1000 + 'k' : String(hz), x + 2, H - 4);
  });
}

// ── Pitch meter helpers (exported for PitchMeter component) ─────────────────

export { hzToPct };

// ── Snapshot: stats computation ──────────────────────────────────────────────

export function computeSnapshotStats(
  pitchLog: PitchPoint[],
  specFrameStore: Float32Array[],
  sampleRate: number
): SnapshotStats {
  const sorted = [...pitchLog.map(p => p.hz)].sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)] ?? 0;
  const pct10  = sorted[Math.floor(sorted.length * 0.10)] ?? sorted[0] ?? 0;
  const pct90  = sorted[Math.floor(sorted.length * 0.90)] ?? sorted[sorted.length - 1] ?? 0;
  const inTarget = pitchLog.filter(p => p.hz >= TARGET_LO && p.hz <= TARGET_HI).length;
  const tgtPct = Math.round(inTarget / pitchLog.length * 100);

  let f2f1Ratio = 0;
  if (specFrameStore.length > 0) {
    const hpb = sampleRate / (2 * specFrameStore[0].length);
    let f1e = 0, f1n = 0, f2e = 0, f2n = 0;
    for (let i = 0; i < specFrameStore[0].length; i++) {
      const hz = i * hpb;
      let sum = 0;
      for (const frame of specFrameStore) sum += frame[i];
      const amp = Math.pow(10, (sum / specFrameStore.length) / 20);
      if (hz >= 300 && hz <= 900)  { f1e += amp; f1n++; }
      if (hz >= 800 && hz <= 2500) { f2e += amp; f2n++; }
    }
    f2f1Ratio = f1n && f2n ? (f2e / f2n) / (f1e / f1n) : 0;
  }

  const ratioLabel = f2f1Ratio >= 1.3 ? 'bright / forward'
    : f2f1Ratio >= 0.9 ? 'moderate' : 'deep / back';

  return { median, tgtPct, pct10, pct90, f2f1Ratio, ratioLabel };
}

// ── Snapshot: pitch timeline canvas ─────────────────────────────────────────

export function renderPitchTimeline(canvas: HTMLCanvasElement, pitchLog: PitchPoint[]): void {
  if (!resizeDPR(canvas) || !pitchLog.length) return;
  const ctx = canvas.getContext('2d')!;
  const W = canvas.clientWidth, H = canvas.clientHeight;
  ctx.clearRect(0, 0, W, H);

  const dur = pitchLog[pitchLog.length - 1].t || 1;

  const yLo = hzToY(TARGET_LO, H), yHi = hzToY(TARGET_HI, H);
  ctx.fillStyle = 'rgba(91,206,250,0.08)';
  ctx.fillRect(0, yHi, W, yLo - yHi);

  [80, 120, 165, 200, 255, 350].forEach(hz => {
    const y = hzToY(hz, H);
    const isEdge = hz === 165 || hz === 255;
    ctx.strokeStyle = isEdge ? 'rgba(91,206,250,0.45)' : '#2a2a4a';
    ctx.lineWidth = 1;
    ctx.setLineDash(isEdge ? [4, 4] : []);
    ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(W, y); ctx.stroke();
    ctx.setLineDash([]);
    ctx.fillStyle = isEdge ? 'rgba(91,206,250,0.8)' : '#6b6b8a';
    ctx.font = '10px system-ui';
    ctx.fillText(hz + ' Hz', 4, y - 3);
  });

  const tStep = dur <= 15 ? 5 : dur <= 60 ? 10 : 30;
  ctx.fillStyle = '#6b6b8a'; ctx.font = '10px system-ui';
  ctx.fillText('0s', 4, H - 5);
  for (let t = tStep; t < dur; t += tStep) {
    const x = (t / dur) * W;
    ctx.strokeStyle = '#2a2a4a'; ctx.lineWidth = 0.5;
    ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, H); ctx.stroke();
    ctx.fillStyle = '#6b6b8a';
    ctx.fillText(t + 's', x + 3, H - 5);
  }

  ctx.strokeStyle = '#F5A9B8'; ctx.lineWidth = 2; ctx.lineJoin = 'round';
  ctx.beginPath();
  let penDown = false;
  for (let i = 0; i < pitchLog.length; i++) {
    const { t, hz } = pitchLog[i];
    if (penDown && t - pitchLog[i - 1].t > 0.25) {
      ctx.stroke(); ctx.beginPath(); penDown = false;
    }
    const x = (t / dur) * W, y = hzToY(hz, H);
    penDown ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
    penDown = true;
  }
  ctx.stroke();

  ctx.fillStyle = '#F5A9B8';
  for (const { t, hz } of pitchLog) {
    ctx.beginPath();
    ctx.arc((t / dur) * W, hzToY(hz, H), 2, 0, Math.PI * 2);
    ctx.fill();
  }
}

// ── Snapshot: F1/F2 formant extraction ──────────────────────────────────────

export interface FormantPoint { f1: number; f2: number; }

export function extractFormantData(
  specFrameStore: Float32Array[],
  sampleRate: number
): FormantPoint[] {
  if (!specFrameStore.length) return [];
  const binCount = specFrameStore[0].length;
  const hpb = sampleRate / (2 * binCount);
  const result: FormantPoint[] = [];

  for (const frame of specFrameStore) {
    let maxE = -Infinity;
    const gLo = Math.round(150 / hpb), gHi = Math.min(Math.round(3000 / hpb), frame.length - 1);
    for (let i = gLo; i <= gHi; i++) if (frame[i] > maxE) maxE = frame[i];
    if (maxE < -52) continue;

    let f1bin = -1, f1max = -Infinity;
    const f1lo = Math.round(300 / hpb), f1hi = Math.min(Math.round(850 / hpb), frame.length - 1);
    for (let i = f1lo; i <= f1hi; i++) if (frame[i] > f1max) { f1max = frame[i]; f1bin = i; }

    let f2bin = -1, f2max = -Infinity;
    const f2lo = Math.round(950 / hpb), f2hi = Math.min(Math.round(2600 / hpb), frame.length - 1);
    for (let i = f2lo; i <= f2hi; i++) if (frame[i] > f2max) { f2max = frame[i]; f2bin = i; }

    if (f1bin < 0 || f2bin < 0) continue;
    result.push({ f1: f1bin * hpb, f2: f2bin * hpb });
  }
  return result;
}

// ── Snapshot: F1/F2 formant dot plot ────────────────────────────────────────

function drawFormantPlotBackground(ctx: CanvasRenderingContext2D, W: number, H: number,
  f2x: (hz: number) => number, f1y: (hz: number) => number) {
  const tzX = f2x(1800);
  ctx.fillStyle = 'rgba(91,206,250,0.07)';
  ctx.fillRect(tzX, 0, W - tzX, H);
  ctx.strokeStyle = 'rgba(91,206,250,0.35)'; ctx.lineWidth = 1; ctx.setLineDash([4, 4]);
  ctx.beginPath(); ctx.moveTo(tzX, 0); ctx.lineTo(tzX, H); ctx.stroke();
  ctx.setLineDash([]);
  ctx.fillStyle = 'rgba(91,206,250,0.55)'; ctx.font = '10px system-ui';
  ctx.fillText('brighter →', tzX + 5, 13);

  [1100, 1400, 1700, 2000, 2300, 2600].forEach(hz => {
    const x = f2x(hz);
    if (x < 0 || x > W) return;
    ctx.strokeStyle = '#2a2a4a'; ctx.lineWidth = 0.5;
    ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, H); ctx.stroke();
    ctx.fillStyle = '#6b6b8a'; ctx.font = '10px system-ui';
    ctx.fillText(hz >= 1000 ? (hz / 1000).toFixed(1) + 'k' : String(hz), x + 2, H - 4);
  });
  [300, 450, 600, 750].forEach(hz => {
    const y = f1y(hz);
    if (y < 0 || y > H) return;
    ctx.strokeStyle = '#2a2a4a'; ctx.lineWidth = 0.5;
    ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(W, y); ctx.stroke();
    ctx.fillStyle = '#6b6b8a'; ctx.font = '10px system-ui';
    ctx.fillText(String(hz), 4, y - 3);
  });

  ctx.fillStyle = '#6b6b8a'; ctx.font = '11px system-ui';
  ctx.textAlign = 'right';
  ctx.fillText('F2 (Hz) →', W - 4, H - 4);
  ctx.textAlign = 'left';
  ctx.save();
  ctx.translate(11, H / 2 + 28); ctx.rotate(-Math.PI / 2);
  ctx.fillText('F1 (Hz)', 0, 0);
  ctx.restore();
  ctx.textAlign = 'left';
}

export function renderFormantPlotFromPoints(
  canvas: HTMLCanvasElement,
  points: FormantPoint[]
): void {
  if (!resizeDPR(canvas)) return;
  const ctx = canvas.getContext('2d')!;
  const W = canvas.clientWidth, H = canvas.clientHeight;
  ctx.clearRect(0, 0, W, H);

  const F2_LO = 900, F2_HI = 2700, F1_LO = 250, F1_HI = 900;
  const f2x = (hz: number) => ((hz - F2_LO) / (F2_HI - F2_LO)) * W;
  const f1y = (hz: number) => H - ((hz - F1_LO) / (F1_HI - F1_LO)) * H;

  drawFormantPlotBackground(ctx, W, H, f2x, f1y);

  if (!points.length) return;

  let sumF1 = 0, sumF2 = 0;
  for (const { f1, f2 } of points) {
    const x = f2x(f2), y = f1y(f1);
    if (x < 0 || x > W || y < 0 || y > H) continue;
    ctx.fillStyle = 'rgba(245,169,184,0.22)';
    ctx.beginPath(); ctx.arc(x, y, 3.5, 0, Math.PI * 2); ctx.fill();
    sumF1 += f1; sumF2 += f2;
  }

  const mx = f2x(sumF2 / points.length), my = f1y(sumF1 / points.length);
  ctx.fillStyle = '#F5A9B8';
  ctx.strokeStyle = '#0d0d24'; ctx.lineWidth = 1.5;
  ctx.beginPath(); ctx.arc(mx, my, 7, 0, Math.PI * 2);
  ctx.fill(); ctx.stroke();
  ctx.fillStyle = '#0d0d24'; ctx.font = 'bold 9px system-ui'; ctx.textAlign = 'center';
  ctx.fillText('μ', mx, my + 3);
  ctx.textAlign = 'left';
}

export function renderFormantPlot(
  canvas: HTMLCanvasElement,
  specFrameStore: Float32Array[],
  sampleRate: number
): void {
  renderFormantPlotFromPoints(canvas, extractFormantData(specFrameStore, sampleRate));
}
