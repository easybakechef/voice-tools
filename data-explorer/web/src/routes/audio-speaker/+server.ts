import { error, redirect } from '@sveltejs/kit';
import { firstKeptClip } from '$lib/server/db';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = ({ url }) => {
  const speaker = url.searchParams.get('speaker');
  if (!speaker) throw error(400, 'missing speaker');
  const path = firstKeptClip(speaker);
  if (!path) throw error(404, 'no kept clip for speaker');
  throw redirect(307, `/audio?path=${encodeURIComponent(path)}`);
};
