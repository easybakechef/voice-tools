export type ActiveType = 'live' | 'clip' | 'file' | null;

export interface PitchPoint {
  t: number;
  hz: number;
}

export interface SnapshotStats {
  median: number;
  tgtPct: number;
  pct10: number;
  pct90: number;
  f2f1Ratio: number;
  ratioLabel: string;
}
