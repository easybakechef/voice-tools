import { samples, datasets } from '$lib/server/db';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ url }) => {
  const p = url.searchParams;
  const num = (k: string) => (p.has(k) && p.get(k) !== '' ? Number(p.get(k)) : undefined);
  const query = {
    q: p.get('q') ?? undefined,
    gender: p.get('gender') ?? 'any',
    dataset: p.get('dataset') ?? 'any',
    keptOnly: p.get('kept') === '1',
    f0min: num('f0min'),
    f0max: num('f0max'),
    limit: 100,
    offset: num('offset') ?? 0
  };
  const { rows, total } = samples(query);
  return { rows, total, query, datasetList: datasets() };
};
