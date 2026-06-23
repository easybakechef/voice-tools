// ============================================================================
//  Resonance community data-access layer
//
//  Public paired recordings for the blind "which take is brighter?" game:
//  list public pairs, cast a vote, read your own vote, aggregate stats, and
//  comment on pairs. Backend-agnostic; reimplement this file to migrate.
// ============================================================================

import { supabase, currentUserId } from '$lib/supabase/client.js';
import type { ResonanceLabel } from './dataset.js';

export interface ResonanceSample {
  id: string;
  label: ResonanceLabel; // hidden by the UI until the user has voted
  storagePath: string;
}

export interface ResonancePair {
  id: string;
  phrase: string;
  authorId: string;
  createdAt: number;
  samples: ResonanceSample[]; // exactly two: one deep, one bright
}

export interface ResonanceStats {
  deep: number;
  bright: number;
  total: number;
}

export interface PairComment {
  id: string;
  authorId: string;
  body: string;
  date: number;
  mine: boolean;
}

/** Public pairs shared by other people, newest first. */
export async function listPublicResonancePairs(limit = 100): Promise<ResonancePair[]> {
  const me = await currentUserId();
  const { data, error } = await supabase
    .from('dataset_pairs')
    .select('id, created_at, speaker_id, phrase:sample_phrases(text), samples:dataset_samples(id, label, storage_path)')
    .eq('visibility', 'public')
    .neq('speaker_id', me)
    .order('created_at', { ascending: false })
    .limit(limit);
  if (error) throw new Error(`Failed to load resonance community: ${error.message}`);

  return (data as any[])
    .map((p) => ({
      id: p.id,
      phrase: p.phrase?.text ?? '(phrase removed)',
      authorId: p.speaker_id,
      createdAt: Date.parse(p.created_at),
      samples: (p.samples ?? []).map((s: any) => ({
        id: s.id,
        label: s.label as ResonanceLabel,
        storagePath: s.storage_path,
      })),
    }))
    .filter((p) => p.samples.length === 2);
}

/** Which sample (if any) the current user already picked as brighter. */
export async function getMyResonanceVote(pairId: string): Promise<string | null> {
  const me = await currentUserId();
  const { data, error } = await supabase
    .from('resonance_votes')
    .select('chosen_sample_id')
    .eq('pair_id', pairId)
    .eq('voter_id', me)
    .maybeSingle();
  if (error) throw new Error(`Could not load your vote: ${error.message}`);
  return data?.chosen_sample_id ?? null;
}

/** Record the user's "brighter" pick for a pair. */
export async function recordResonanceVote(pairId: string, chosenSampleId: string): Promise<void> {
  const me = await currentUserId();
  const { error } = await supabase
    .from('resonance_votes')
    .insert({ pair_id: pairId, voter_id: me, chosen_sample_id: chosenSampleId });
  if (error) throw new Error(`Could not record vote: ${error.message}`);
}

/** Aggregate vote counts per speaker-label for a pair. */
export async function getResonanceStats(pairId: string): Promise<ResonanceStats> {
  await currentUserId();
  const { data, error } = await supabase.rpc('resonance_pair_stats', { p_pair_id: pairId });
  if (error) throw new Error(`Could not load stats: ${error.message}`);
  const rows = (data as { label: ResonanceLabel; votes: number }[]) ?? [];
  const deep = Number(rows.find((r) => r.label === 'deep')?.votes ?? 0);
  const bright = Number(rows.find((r) => r.label === 'bright')?.votes ?? 0);
  return { deep, bright, total: deep + bright };
}

// ── Pair comments ───────────────────────────────────────────────────────────
export async function listPairComments(pairId: string): Promise<PairComment[]> {
  const me = await currentUserId();
  const { data, error } = await supabase
    .from('pair_comments')
    .select('id, author_id, body, created_at')
    .eq('pair_id', pairId)
    .order('created_at', { ascending: true });
  if (error) throw new Error(`Failed to load comments: ${error.message}`);
  return (data as any[]).map((c) => ({
    id: c.id,
    authorId: c.author_id,
    body: c.body,
    date: Date.parse(c.created_at),
    mine: c.author_id === me,
  }));
}

export async function addPairComment(pairId: string, body: string): Promise<PairComment> {
  const me = await currentUserId();
  const trimmed = body.trim();
  if (!trimmed) throw new Error('Comment is empty');
  const { data, error } = await supabase
    .from('pair_comments')
    .insert({ pair_id: pairId, author_id: me, body: trimmed })
    .select('id, author_id, body, created_at')
    .single();
  if (error || !data) throw new Error(`Could not post comment: ${error?.message ?? 'unknown'}`);
  return { id: data.id, authorId: data.author_id, body: data.body, date: Date.parse(data.created_at), mine: true };
}

export async function deletePairComment(id: string): Promise<void> {
  const { error } = await supabase.from('pair_comments').delete().eq('id', id);
  if (error) throw new Error(`Could not delete comment: ${error.message}`);
}

export function pairAuthorLabel(c: PairComment): string {
  return c.mine ? 'You' : `Anon ${c.authorId.slice(0, 6)}`;
}
