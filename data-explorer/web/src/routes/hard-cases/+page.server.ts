import { hardCases } from '$lib/server/db';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ url }) => {
  const mode = (url.searchParams.get('mode') as 'wrong' | 'ambiguous' | 'all') ?? 'all';
  const gender = url.searchParams.get('gender') ?? 'any';
  const model = (url.searchParams.get('model') as 'resonance' | 'combo') ?? 'resonance';
  const res = hardCases({ mode, gender, model, limit: 300 });
  return { ...res, mode, gender };
};
