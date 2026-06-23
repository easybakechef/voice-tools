export const FFT_SIZE  = 4096;
export const SMOOTH    = 0.8;
export const MIN_HZ    = 80;
export const MAX_HZ    = 350;
export const BAR_MIN   = 60;
export const BAR_MAX   = 4000;
export const CLIP_DUR  = 30;
export const TARGET_LO = 165;
export const TARGET_HI = 255;

export const FORMANT_BANDS = [
  { label: 'F0', lo: 80,   hi: 350,  color: '#e74c6f' },
  { label: 'F1', lo: 300,  hi: 900,  color: '#f39c12' },
  { label: 'F2', lo: 800,  hi: 2500, color: '#5BCEFA' },
  { label: 'F3', lo: 2000, hi: 3500, color: '#9b59b6' },
] as const;

export const CLIPS = {
  fem: {
    label: 'Karen Savage — Anne of Green Gables',
    source: 'LibriVox / Public Domain',
    desc: 'Natural feminine voice. Watch for higher fundamental pitch and energy in the F2 band (2–2.5 kHz) — the "brightness" cue for feminine speech.',
    url: 'https://archive.org/download/anne_greengables_librivox/anne_of_green_gables_01_montgomery.mp3',
  },
  masc: {
    label: 'Bryan Ness — "Escape" by E.F. Benson',
    source: 'LibriVox / Public Domain',
    desc: 'Natural masculine voice. Notice the lower pitch and F2 energy sitting below 1.5 kHz — compare the spectrum against the feminine clip.',
    url: 'https://archive.org/download/nonfiction001_librivox/escape_benson_bn.mp3',
  },
} as const;

export type ClipKey = keyof typeof CLIPS;
