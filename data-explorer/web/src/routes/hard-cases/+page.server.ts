import { hardCases } from '$lib/server/db';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ url }) => {
  const mode = (url.searchParams.get('mode') as 'wrong' | 'ambiguous' | 'all') ?? 'all';
  const gender = url.searchParams.get('gender') ?? 'any';
  const res = hardCases({ mode, gender, limit: 300 });
  return { ...res, mode, gender };
};
