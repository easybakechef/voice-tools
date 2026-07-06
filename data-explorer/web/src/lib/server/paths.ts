import { resolve } from 'node:path';

// Both `vite dev` and `node build/index.js` run with cwd = web/.
// data-explorer/data holds samples.db and the kept/ clips.
export const DATA_ROOT = process.env.DATA_ROOT
  ? resolve(process.env.DATA_ROOT)
  : resolve(process.cwd(), '../data');

export const DB_PATH = process.env.SAMPLES_DB
  ? resolve(process.env.SAMPLES_DB)
  : resolve(DATA_ROOT, 'samples.db');

export const KEPT_DIR = resolve(DATA_ROOT, 'kept');
