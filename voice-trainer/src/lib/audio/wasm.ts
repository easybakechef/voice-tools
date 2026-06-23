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

  let bestTau = -1, bestVal = -Infinity;
  for (let tau = minP; tau <= maxP; tau++) {
    const r = nsdf(buf, tau);
    if (r > bestVal) { bestVal = r; bestTau = tau; }
  }
  if (bestVal < 0.4 || bestTau < 0) return null;

  const a = bestTau > 0             ? nsdf(buf, bestTau - 1) : 0;
  const b =                           nsdf(buf, bestTau);
  const c = bestTau < buf.length - 1 ? nsdf(buf, bestTau + 1) : 0;
  const denom = a - 2 * b + c;
  const refined = denom !== 0 ? bestTau - 0.5 * (a - c) / denom : bestTau;
  return sr / refined;
}
