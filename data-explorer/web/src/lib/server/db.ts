import Database from 'better-sqlite3';
import { existsSync } from 'node:fs';
import { DB_PATH } from './paths';

let _db: Database.Database | null = null;
function db(): Database.Database {
  if (!_db) {
    if (!existsSync(DB_PATH)) throw new Error(`samples.db not found at ${DB_PATH}`);
    _db = new Database(DB_PATH, { readonly: true, fileMustExist: true });
  }
  return _db;
}

function hasResonance(): boolean {
  const row = db()
    .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='resonance'")
    .get();
  return !!row;
}

export type Overview = {
  total: number;
  kept: number;
  speakers: number;
  byGender: { gender: string; n: number }[];
  f0Hist: { bin: number; male: number; female: number }[];
  resonance?: {
    speakers: number;
    matched: number;
    vtlMale: number;
    vtlFemale: number;
    accuracy: number;
    auc: number;
    fullAuc: number;
    vtlAuc: number;
    hardCount: number;
    comboAuc: number;
    comboFullAuc: number;
    comboAccuracy: number;
    comboHardCount: number;
  };
};

export function overview(): Overview {
  const d = db();
  const total = (d.prepare('SELECT count(*) n FROM samples').get() as any).n;
  const kept = (d.prepare('SELECT count(*) n FROM samples WHERE kept=1').get() as any).n;
  const speakers = (d.prepare('SELECT count(DISTINCT speaker) n FROM samples').get() as any).n;
  const byGender = d
    .prepare("SELECT coalesce(gender,'unknown') gender, count(*) n FROM samples GROUP BY gender ORDER BY n DESC")
    .all() as { gender: string; n: number }[];

  // F0 histogram 60..320 Hz, 20 Hz bins, split by gender
  const rows = d
    .prepare(
      "SELECT gender, f0_median f0 FROM samples WHERE f0_median BETWEEN 60 AND 320 AND gender IN ('male','female')"
    )
    .all() as { gender: string; f0: number }[];
  const bins = new Map<number, { male: number; female: number }>();
  for (let b = 60; b < 320; b += 20) bins.set(b, { male: 0, female: 0 });
  for (const r of rows) {
    const b = Math.floor(r.f0 / 20) * 20;
    const cell = bins.get(b);
    if (cell) cell[r.gender as 'male' | 'female']++;
  }
  const f0Hist = [...bins.entries()].map(([bin, v]) => ({ bin, ...v }));

  const out: Overview = { total, kept, speakers, byGender, f0Hist };

  if (hasResonance()) {
    const r = d
      .prepare(
        `SELECT count(*) speakers,
                sum(in_matched) matched,
                avg(CASE WHEN gender='male' THEN vtl END) vtlMale,
                avg(CASE WHEN gender='female' THEN vtl END) vtlFemale,
                avg(correct) accuracy,
                sum(CASE WHEN correct=0 THEN 1 ELSE 0 END) hardCount,
                avg(combo_correct) comboAccuracy,
                sum(CASE WHEN combo_correct=0 THEN 1 ELSE 0 END) comboHardCount
         FROM resonance`
      )
      .get() as any;
    const meta = (d.prepare('SELECT * FROM resonance_meta LIMIT 1').get() as any) ?? {};
    out.resonance = {
      speakers: r.speakers,
      matched: r.matched ?? 0,
      vtlMale: r.vtlMale,
      vtlFemale: r.vtlFemale,
      accuracy: r.accuracy,
      auc: meta.auc ?? 0,
      fullAuc: meta.full_auc ?? 0,
      vtlAuc: meta.vtl_auc ?? 0,
      hardCount: r.hardCount,
      comboAuc: meta.combo_auc ?? 0,
      comboFullAuc: meta.combo_full_auc ?? 0,
      comboAccuracy: r.comboAccuracy,
      comboHardCount: r.comboHardCount
    };
  }
  return out;
}

export type SampleRow = {
  id: number;
  dataset: string;
  speaker: string;
  gender: string | null;
  path: string;
  duration: number | null;
  f0_median: number | null;
  f1_median: number | null;
  f2_median: number | null;
  voiced_frac: number | null;
  kept: number;
};

export type SampleQuery = {
  q?: string;
  gender?: string;
  dataset?: string;
  keptOnly?: boolean;
  f0min?: number;
  f0max?: number;
  limit?: number;
  offset?: number;
};

export function samples(qy: SampleQuery): { rows: SampleRow[]; total: number } {
  const where: string[] = [];
  const args: any[] = [];
  if (qy.q) {
    where.push('(speaker LIKE ? OR path LIKE ?)');
    args.push(`%${qy.q}%`, `%${qy.q}%`);
  }
  if (qy.gender && qy.gender !== 'any') {
    where.push('gender = ?');
    args.push(qy.gender);
  }
  if (qy.dataset && qy.dataset !== 'any') {
    where.push('dataset = ?');
    args.push(qy.dataset);
  }
  if (qy.keptOnly) where.push('kept = 1');
  if (qy.f0min != null) {
    where.push('f0_median >= ?');
    args.push(qy.f0min);
  }
  if (qy.f0max != null) {
    where.push('f0_median <= ?');
    args.push(qy.f0max);
  }
  const clause = where.length ? `WHERE ${where.join(' AND ')}` : '';
  const total = (db().prepare(`SELECT count(*) n FROM samples ${clause}`).get(...args) as any).n;
  const limit = Math.min(qy.limit ?? 100, 500);
  const rows = db()
    .prepare(
      `SELECT id, dataset, speaker, gender, path, duration, f0_median, f1_median, f2_median, voiced_frac, kept
       FROM samples ${clause} ORDER BY speaker, path LIMIT ? OFFSET ?`
    )
    .all(...args, limit, qy.offset ?? 0) as SampleRow[];
  return { rows, total };
}

export function firstKeptClip(speaker: string): string | null {
  const row = db()
    .prepare('SELECT path FROM samples WHERE speaker = ? AND kept = 1 ORDER BY path LIMIT 1')
    .get(speaker) as { path: string } | undefined;
  return row?.path ?? null;
}

export function datasets(): string[] {
  return (db().prepare('SELECT DISTINCT dataset FROM samples ORDER BY dataset').all() as any[]).map(
    (r) => r.dataset
  );
}

export function speakerDetail(speaker: string) {
  const clips = db()
    .prepare(
      `SELECT id, dataset, path, duration, f0_median, f1_median, f2_median, voiced_frac, kept
       FROM samples WHERE speaker = ? ORDER BY path`
    )
    .all(speaker) as SampleRow[];
  let resonance = null;
  if (hasResonance()) {
    resonance = db().prepare('SELECT * FROM resonance WHERE speaker = ?').get(speaker) ?? null;
  }
  return { speaker, clips, resonance };
}

export type ResonanceRow = {
  speaker: string;
  gender: string;
  f0: number;
  f1: number;
  f2: number;
  f3: number;
  f4: number;
  f5: number;
  vtl: number;
  tilt: number;
  h1h2: number;
  prob_female: number;
  pred: string;
  correct: number;
  margin: number;
  in_matched: number;
  combo_prob: number;
  combo_pred: string;
  combo_correct: number;
  combo_margin: number;
};

export type HardModel = 'resonance' | 'combo';

/** Samples the metric struggles on: misclassified or low-margin (ambiguous).
 *  `model` chooses which classifier: 'vtl' (resonance only) or 'combo' (pitch+resonance). */
export function hardCases(opts: {
  mode?: 'wrong' | 'ambiguous' | 'all';
  gender?: string;
  model?: HardModel;
  limit?: number;
}): {
  rows: ResonanceRow[];
  available: boolean;
  model: HardModel;
  auc: number;
  fullAuc: number;
  threshold: number;
} {
  if (!hasResonance())
    return { rows: [], available: false, model: 'resonance', auc: 0, fullAuc: 0, threshold: 0 };
  const meta = (db().prepare('SELECT * FROM resonance_meta LIMIT 1').get() as any) ?? {};
  const model: HardModel = opts.model === 'combo' ? 'combo' : 'resonance';
  const correctCol = model === 'combo' ? 'combo_correct' : 'correct';
  const marginCol = model === 'combo' ? 'combo_margin' : 'margin';

  const where: string[] = [];
  const args: any[] = [];
  if (opts.mode === 'wrong') where.push(`${correctCol} = 0`);
  else if (opts.mode === 'ambiguous') where.push(`${marginCol} <= 0.15`);
  else where.push(`(${correctCol} = 0 OR ${marginCol} <= 0.15)`);
  if (opts.gender && opts.gender !== 'any') {
    where.push('gender = ?');
    args.push(opts.gender);
  }
  const rows = db()
    .prepare(
      `SELECT * FROM resonance WHERE ${where.join(' AND ')}
       ORDER BY ${marginCol} ASC LIMIT ?`
    )
    .all(...args, Math.min(opts.limit ?? 200, 1000)) as ResonanceRow[];
  return {
    rows,
    available: true,
    model,
    auc: model === 'combo' ? meta.combo_auc ?? 0 : meta.auc ?? 0,
    fullAuc: model === 'combo' ? meta.combo_full_auc ?? 0 : meta.full_auc ?? 0,
    threshold: meta.threshold ?? 0
  };
}
