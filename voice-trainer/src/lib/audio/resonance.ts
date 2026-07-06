// ============================================================================
//  Resonance scoring — a single "brightness" value from formants.
//
//  Method adapted (clean-room) from L. McNulty's acousticgender.space writeup
//  (https://lmcnulty.me/words/how-to-calculate-resonance/): express each formant
//  as how many standard deviations it sits above/below the average FOR THAT
//  VOWEL, combine with fixed weights (F1 dominant), and map to a percentage
//  where 50% = average, 100% ≈ +2σ (brighter), 0% ≈ −2σ (deeper).
//
//  Their tool knows the vowel via forced alignment to a known transcript. For a
//  LIVE bar we don't know the text, so we classify the nearest vowel on the fly
//  from F1/F2. That makes the live value approximate (and slightly centre-biased,
//  since the nearest vowel minimises deviation) — good as a real-time cue, not a
//  lab measurement. The accurate version belongs on recorded/known-text audio.
// ============================================================================

// Per-vowel reference: neutral (blended adult) F1/F2 means + standard deviations
// in Hz. Approximate published English monophthong values — tunable / calibratable.
interface VowelRef { label: string; f1: number; f2: number; f1sd: number; f2sd: number; }

const VOWELS: VowelRef[] = [
  { label: 'i (heed)',    f1: 340, f2: 2300, f1sd: 70,  f2sd: 250 },
  { label: 'ɪ (hid)',     f1: 460, f2: 1990, f1sd: 80,  f2sd: 250 },
  { label: 'ɛ (head)',    f1: 580, f2: 1800, f1sd: 90,  f2sd: 220 },
  { label: 'æ (had)',     f1: 690, f2: 1750, f1sd: 100, f2sd: 220 },
  { label: 'ɑ (hot)',     f1: 730, f2: 1200, f1sd: 100, f2sd: 200 },
  { label: 'ɔ (bought)',  f1: 620, f2: 1000, f1sd: 90,  f2sd: 180 },
  { label: 'ʌ (hut)',     f1: 640, f2: 1300, f1sd: 90,  f2sd: 200 },
  { label: 'ʊ (hood)',    f1: 470, f2: 1150, f1sd: 80,  f2sd: 200 },
  { label: 'u (who)',     f1: 360, f2: 1100, f1sd: 70,  f2sd: 250 },
  { label: 'ɝ (heard)',   f1: 480, f2: 1350, f1sd: 80,  f2sd: 200 },
];

// Weights from the acousticgender writeup: F1 dominant, F2 minor, F3 ignored.
const W_F1 = 0.732;
const W_F2 = 0.268;

// Sensitivity: how many σ fill the 0–100% bar. Smaller = punchier / more
// dynamic range for real voices (which sit ~±1σ from the neutral reference).
const SIGMA_RANGE = 1.3;

// Vowel-classification stickiness (in σ² units). The live nearest-vowel guess
// keeps the previous vowel unless another beats it by more than this margin —
// stops the score "resetting" when you brighten a sustained vowel across a
// category boundary.
const STICKY_MARGIN = 1.5;

export interface ResonanceResult {
  pct: number;       // 0–100, 50 = average for the vowel
  vowel: string;     // nearest-classified vowel label
}

/**
 * Resonance score from a frame's F1/F2 (Hz). `prevVowel` (the last frame's
 * classified vowel) adds hysteresis so the score doesn't reset on boundary
 * flips. Returns null for implausible input.
 */
export function computeResonance(
  f1: number,
  f2: number,
  prevVowel?: string | null,
): ResonanceResult | null {
  if (!(f1 > 0) || !(f2 > 0)) return null;

  const dist = (v: VowelRef) => {
    const z1 = (f1 - v.f1) / v.f1sd;
    const z2 = (f2 - v.f2) / v.f2sd;
    return z1 * z1 + z2 * z2;
  };

  // Classify: nearest vowel by normalised distance in F1/F2 space.
  let best: VowelRef | null = null;
  let bestD = Infinity;
  for (const v of VOWELS) {
    const d = dist(v);
    if (d < bestD) { bestD = d; best = v; }
  }
  if (!best) return null;

  // Hysteresis: keep the previous vowel if it's still a close match.
  if (prevVowel && prevVowel !== best.label) {
    const prev = VOWELS.find((v) => v.label === prevVowel);
    if (prev && dist(prev) <= bestD + STICKY_MARGIN) best = prev;
  }

  // How far above/below the average for that vowel (in σ).
  const z1 = (f1 - best.f1) / best.f1sd;
  const z2 = (f2 - best.f2) / best.f2sd;
  const weightedZ = W_F1 * z1 + W_F2 * z2;

  // Map σ → percentage (50% = average, ±SIGMA_RANGE → 0/100).
  const pct = Math.max(0, Math.min(100, ((weightedZ + SIGMA_RANGE) / (2 * SIGMA_RANGE)) * 100));
  return { pct, vowel: best.label };
}
