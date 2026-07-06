import { error } from '@sveltejs/kit';
import { existsSync, readFileSync } from 'node:fs';
import { resolve, basename } from 'node:path';
import { DATA_ROOT, KEPT_DIR } from '$lib/server/paths';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = ({ url }) => {
  const raw = url.searchParams.get('path');
  if (!raw) throw error(400, 'missing path');

  // resolve candidates, all constrained to DATA_ROOT
  const candidates = [resolve(raw), resolve(KEPT_DIR, basename(raw))];
  const file = candidates.find((c) => c.startsWith(DATA_ROOT) && existsSync(c));
  if (!file) throw error(404, 'audio not available on disk');

  const buf = readFileSync(file);
  return new Response(buf, {
    headers: {
      'content-type': file.endsWith('.wav') ? 'audio/wav' : 'audio/flac',
      'content-length': String(buf.length),
      'cache-control': 'public, max-age=3600'
    }
  });
};
