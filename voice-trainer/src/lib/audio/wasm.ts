import { MIN_HZ, MAX_HZ } from './constants.js';

type PitchFn = (samples: Float32Array, sample_rate: number) => number;
let wasmFn: PitchFn | null = null;

export async function tryLoadWasm(): Promise<void> {
  try {
    const mod = await import('$lib/wasm/voice_dsp/voice_dsp.js');
    // vite-plugin-wasm handles init automatically with bundler target,
    // but calling default() is safe as a no-op if already initialized.
    if (typeof mod.default === 'function') await mod.default();
    wasmFn = mod.detect_pitch as PitchFn;
    console.info('[voice-dsp] WASM pitch detection active');
  } catch {
    console.info('[voice-dsp] WASM not built — using JS fallback. Run `npm run wasm` to enable.');
  }
}

export function detectPitch(buf: Float32Array, sr: number): number | null {
  if (wasmFn) {
    const r = wasmFn(buf, sr);
    return r > 0 ? r : null;
  }
  return detectPitchJS(buf, sr);
}

// ── JS fallback: NSDF with parabolic interpolation ──────────────────────────

function nsdf(buf: Float32Array, tau: number): number {
  let num = 0, den = 0;
  for (let i = 0; i < buf.length - tau; i++) {
    num += buf[i] * buf[i + tau];
    den += buf[i] * buf[i] + buf[i + tau] * buf[i + tau];
  }
  return den === 0 ? 0 : 2 * num / den;
}

function detectPitchJS(buf: Float32Array, sr: number): number | null {
  const minP = Math.floor(sr / MAX_HZ);
  const maxP = Math.ceil(sr / MIN_HZ);

  let rms = 0;
  for (let i = 0; i < buf.length; i++) rms += buf[i] * buf[i];
  if (Math.sqrt(rms / buf.length) < 0.01) return null;

  // NSDF across the search range.
  const n = maxP - minP + 1;
  const r = new Float32Array(n);
  for (let t = minP; t <= maxP; t++) r[t - minP] = nsdf(buf, t);

  // Collect local maxima and the overall peak value. (McLeod Pitch Method:
  // octave errors come from taking the *tallest* peak — which is often the
  // period-doubled one. Instead, take the FIRST peak that's nearly as tall.)
  let maxVal = -Infinity;
  const peaks: number[] = [];
  for (let i = 1; i < n - 1; i++) {
    if (r[i] > r[i - 1] && r[i] >= r[i + 1]) {
      peaks.push(i);
      if (r[i] > maxVal) maxVal = r[i];
    }
  }
  if (peaks.length === 0 || maxVal < 0.4) return null;

  const thresh = 0.88 * maxVal;
  let chosen = peaks[0];
  for (const p of peaks) {
    if (r[p] >= thresh) { chosen = p; break; }
  }

  // Parabolic interpolation around the chosen peak.
  const a = chosen > 0     ? r[chosen - 1] : r[chosen];
  const b =                  r[chosen];
  const c = chosen < n - 1 ? r[chosen + 1] : r[chosen];
  const denom = a - 2 * b + c;
  const offset = denom !== 0 ? -0.5 * (a - c) / denom : 0;
  const tau = (minP + chosen) + offset;
  return tau > 0 ? sr / tau : null;
}
