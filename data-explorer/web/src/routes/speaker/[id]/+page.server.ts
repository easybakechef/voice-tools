import { speakerDetail } from '$lib/server/db';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
  return speakerDetail(params.id);
};
