// ============================================================================
//  Ranking / pairwise-comparison data-access layer
//
//  Backend-agnostic interface for comparison sets and votes. Components import
//  these functions only; reimplement this file to migrate backends.
// ============================================================================

import { supabase, currentUserId } from '$lib/supabase/client.js';
import {
  PUBLIC_RECORDING_COLUMNS,
  toPublicRecording,
  type PublicRecording,
  type RecordingRow,
} from './recordings.js';

export interface ComparisonSet {
  id: string;
  name: string;
  description: string | null;
  createdAt: number;
  itemCount: number;
}

export interface RankingRow {
  recordingId: string;
  name: string;
  wins: number;
  comparisons: number;
  winRate: number;
}

/** List all comparison sets (with item counts), newest first. */
export async function listSets(): Promise<ComparisonSet[]> {
  await currentUserId();
  const { data, error } = await supabase
    .from('comparison_sets')
    .select('id, name, description, created_at, comparison_items(count)')
    .order('created_at', { ascending: false });
  if (error) throw new Error(`Failed to load comparison sets: ${error.message}`);

  return (data as any[]).map((s) => ({
    id: s.id,
    name: s.name,
    description: s.description,
    createdAt: Date.parse(s.created_at),
    itemCount: s.comparison_items?.[0]?.count ?? 0,
  }));
}

/** Create a set and add the given (public) recordings to it. Returns set id. */
export async function createSet(
  name: string,
  description: string,
  recordingIds: string[],
): Promise<string> {
  const creatorId = await currentUserId();
  if (recordingIds.length < 2) throw new Error('A set needs at least 2 recordings');

  const { data: set, error: sErr } = await supabase
    .from('comparison_sets')
    .insert({ creator_id: creatorId, name: name.trim() || 'Untitled set', description: description.trim() || null })
    .select('id')
    .single();
  if (sErr || !set) throw new Error(`Could not create set: ${sErr?.message ?? 'unknown'}`);

  const rows = recordingIds.map((rid) => ({ set_id: set.id, recording_id: rid }));
  const { error: iErr } = await supabase.from('comparison_items').insert(rows);
  if (iErr) {
    // Roll back the empty set so we don't leave a broken one behind.
    await supabase.from('comparison_sets').delete().eq('id', set.id);
    throw new Error(`Could not add recordings to set: ${iErr.message}`);
  }
  return set.id as string;
}

/** Recordings that belong to a set (only those still public/visible). */
export async function getSetItems(setId: string): Promise<PublicRecording[]> {
  await currentUserId();
  const { data, error } = await supabase
    .from('comparison_items')
    .select(`recording:recordings(${PUBLIC_RECORDING_COLUMNS})`)
    .eq('set_id', setId);
  if (error) throw new Error(`Failed to load set items: ${error.message}`);

  // `recording` is a to-one embed (single object at runtime); PostgREST's types
  // widen it to an array, so cast through unknown.
  return (data as unknown as { recording: (RecordingRow & { user_id: string }) | null }[])
    .map((row) => row.recording)
    .filter((r): r is RecordingRow & { user_id: string } => r != null)
    .map(toPublicRecording);
}

/** One of the current user's votes (used to skip already-judged pairs). */
export interface MyVote {
  id: string;
  a: string;
  b: string;
}

/** The current user's votes for a set, so the arena can skip judged pairs. */
export async function getMyVotes(setId: string): Promise<MyVote[]> {
  const voterId = await currentUserId();
  const { data, error } = await supabase
    .from('comparison_votes')
    .select('id, recording_a, recording_b')
    .eq('set_id', setId)
    .eq('voter_id', voterId);
  if (error) throw new Error(`Could not load your votes: ${error.message}`);
  return (data as any[]).map((r) => ({ id: r.id, a: r.recording_a, b: r.recording_b }));
}

/** Record one pairwise judgment: `winner` (a or b) is "more <attribute>". Returns the vote id. */
export async function recordVote(args: {
  setId: string;
  recordingA: string;
  recordingB: string;
  winnerId: string;
  attribute?: string;
}): Promise<string> {
  const voterId = await currentUserId();
  const { data, error } = await supabase
    .from('comparison_votes')
    .insert({
      set_id: args.setId,
      recording_a: args.recordingA,
      recording_b: args.recordingB,
      winner_id: args.winnerId,
      attribute: args.attribute ?? 'feminine',
      voter_id: voterId,
    })
    .select('id')
    .single();
  if (error || !data) throw new Error(`Could not record vote: ${error?.message ?? 'unknown'}`);
  return data.id as string;
}

/** Undo a vote (the voter's own, per RLS) — powers the Undo button. */
export async function deleteVote(id: string): Promise<void> {
  const { error } = await supabase.from('comparison_votes').delete().eq('id', id);
  if (error) throw new Error(`Could not undo vote: ${error.message}`);
}

/** Aggregate win-rates for a set (via the rank_set SQL function). */
export async function getSetRankings(setId: string): Promise<RankingRow[]> {
  await currentUserId();
  const { data, error } = await supabase.rpc('rank_set', { p_set_id: setId });
  if (error) throw new Error(`Could not load rankings: ${error.message}`);
  return (data as any[]).map((r) => ({
    recordingId: r.recording_id,
    name: r.name,
    wins: Number(r.wins),
    comparisons: Number(r.comparisons),
    winRate: Number(r.win_rate),
  }));
}
