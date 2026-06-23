// ============================================================================
//  Recordings data-access layer  (backend-agnostic interface)
//
//  This is the ONLY place the app talks to Supabase for recordings. Components
//  import these functions and the SavedRecording type — never the Supabase
//  client directly. To migrate to a different backend later, reimplement this
//  one file (and src/lib/supabase/client.ts); the rest of the app is unaffected.
// ============================================================================

import { supabase, currentUserId } from '$lib/supabase/client.js';
import type { PitchPoint, SnapshotStats } from '$lib/audio/types.js';
import type { FormantPoint } from '$lib/audio/drawing.js';

export type { FormantPoint };

const BUCKET = 'recordings';
const SIGNED_URL_TTL = 60 * 60; // seconds

export type Visibility = 'private' | 'public';

/** App-facing recording shape (storage details intentionally hidden). */
export interface SavedRecording {
  id: string;
  name: string;
  date: number; // epoch ms
  duration: number;
  medianPitch: number;
  visibility: Visibility;
  pitchLog: PitchPoint[];
  formantData: FormantPoint[];
  stats: SnapshotStats | null;
}

// Shape of a row as returned by Supabase (snake_case columns).
export interface RecordingRow {
  id: string;
  name: string;
  recorded_at: string;
  duration: number;
  median_pitch: number;
  storage_path: string;
  visibility: Visibility;
  pitch_log: PitchPoint[];
  formant_data: FormantPoint[];
  stats: SnapshotStats | null;
}

export const RECORDING_COLUMNS =
  'id, name, recorded_at, duration, median_pitch, storage_path, visibility, pitch_log, formant_data, stats';

/** Column list for a recording plus its owner — for public/embedded queries. */
export const PUBLIC_RECORDING_COLUMNS = `${RECORDING_COLUMNS}, user_id`;

export function rowToRecording(r: RecordingRow): SavedRecording {
  return {
    id: r.id,
    name: r.name,
    date: Date.parse(r.recorded_at),
    duration: r.duration,
    medianPitch: r.median_pitch,
    visibility: r.visibility,
    pitchLog: r.pitch_log ?? [],
    formantData: r.formant_data ?? [],
    stats: r.stats ?? null,
  };
}

/**
 * Save a recording: upload the audio to private Storage, then insert the row.
 * Returns the new recording id. Throws on quota violation (and cleans up the
 * orphaned upload).
 */
export async function saveRecording(rec: {
  name: string;
  date: number;
  duration: number;
  medianPitch: number;
  blob: Blob;
  pitchLog: PitchPoint[];
  formantData: FormantPoint[];
  stats: SnapshotStats;
}): Promise<string> {
  const userId = await currentUserId();
  const path = `${userId}/${crypto.randomUUID()}.webm`;

  const { error: upErr } = await supabase.storage
    .from(BUCKET)
    .upload(path, rec.blob, { contentType: rec.blob.type || 'audio/webm', upsert: false });
  if (upErr) throw new Error(`Audio upload failed: ${upErr.message}`);

  const { data, error: insErr } = await supabase
    .from('recordings')
    .insert({
      user_id: userId,
      name: rec.name || 'Untitled',
      recorded_at: new Date(rec.date).toISOString(),
      duration: rec.duration,
      median_pitch: rec.medianPitch,
      storage_path: path,
      size_bytes: rec.blob.size,
      pitch_log: rec.pitchLog,
      formant_data: rec.formantData,
      stats: rec.stats,
    })
    .select('id')
    .single();

  if (insErr || !data) {
    // Roll back the orphaned upload so storage stays consistent with the table.
    await supabase.storage.from(BUCKET).remove([path]);
    throw new Error(`Save failed: ${insErr?.message ?? 'unknown error'}`);
  }
  return data.id as string;
}

/** List the current user's own recordings, newest first. */
export async function listRecordings(): Promise<SavedRecording[]> {
  const userId = await currentUserId();
  const { data, error } = await supabase
    .from('recordings')
    .select(RECORDING_COLUMNS)
    .eq('user_id', userId)
    .order('recorded_at', { ascending: false });

  if (error) throw new Error(`Failed to load recordings: ${error.message}`);
  return (data as RecordingRow[]).map(rowToRecording);
}

/** Delete a recording and its audio file. */
export async function deleteRecording(id: string): Promise<void> {
  const { data, error: selErr } = await supabase
    .from('recordings')
    .select('storage_path')
    .eq('id', id)
    .single();
  if (selErr) throw new Error(`Delete failed: ${selErr.message}`);

  const { error: delErr } = await supabase.from('recordings').delete().eq('id', id);
  if (delErr) throw new Error(`Delete failed: ${delErr.message}`);

  // Best-effort removal of the blob; the row (source of truth) is already gone.
  if (data?.storage_path) {
    await supabase.storage.from(BUCKET).remove([data.storage_path]);
  }
}

/** Mark a recording public (shared for feedback) or private. Owner-only via RLS. */
export async function setRecordingVisibility(id: string, visibility: Visibility): Promise<void> {
  const { error } = await supabase
    .from('recordings')
    .update({ visibility })
    .eq('id', id);
  if (error) throw new Error(`Could not update visibility: ${error.message}`);
}

/** A public recording shared by someone else, for the community feedback feed. */
export interface PublicRecording extends SavedRecording {
  authorId: string;
}

/** Map a recordings row (with user_id) to the public-facing shape. */
export function toPublicRecording(r: RecordingRow & { user_id: string }): PublicRecording {
  return { ...rowToRecording(r), authorId: r.user_id };
}

/** All public recordings, including the current user's own — for set-building. */
export async function listAllPublicRecordings(limit = 200): Promise<PublicRecording[]> {
  await currentUserId();
  const { data, error } = await supabase
    .from('recordings')
    .select(PUBLIC_RECORDING_COLUMNS)
    .eq('visibility', 'public')
    .order('recorded_at', { ascending: false })
    .limit(limit);
  if (error) throw new Error(`Failed to load public recordings: ${error.message}`);
  return (data as (RecordingRow & { user_id: string })[]).map(toPublicRecording);
}

/** List other people's public recordings, newest first. */
export async function listPublicRecordings(limit = 100): Promise<PublicRecording[]> {
  const userId = await currentUserId();
  const { data, error } = await supabase
    .from('recordings')
    .select(`${RECORDING_COLUMNS}, user_id`)
    .eq('visibility', 'public')
    .neq('user_id', userId)
    .order('recorded_at', { ascending: false })
    .limit(limit);

  if (error) throw new Error(`Failed to load community recordings: ${error.message}`);
  return (data as (RecordingRow & { user_id: string })[]).map((r) => ({
    ...rowToRecording(r),
    authorId: r.user_id,
  }));
}

/** The current user's own PUBLIC recordings, newest first. */
export async function listMyPublicRecordings(limit = 100): Promise<PublicRecording[]> {
  const userId = await currentUserId();
  const { data, error } = await supabase
    .from('recordings')
    .select(`${RECORDING_COLUMNS}, user_id`)
    .eq('visibility', 'public')
    .eq('user_id', userId)
    .order('recorded_at', { ascending: false })
    .limit(limit);

  if (error) throw new Error(`Failed to load your shared recordings: ${error.message}`);
  return (data as (RecordingRow & { user_id: string })[]).map(toPublicRecording);
}

/**
 * Get a short-lived, playable URL for a recording's audio. Async because the
 * bucket is private — we mint a signed URL rather than expose a public link.
 */
export async function getAudioUrl(id: string): Promise<string> {
  const { data: row, error: selErr } = await supabase
    .from('recordings')
    .select('storage_path')
    .eq('id', id)
    .single();
  if (selErr || !row) throw new Error(`Audio not found: ${selErr?.message ?? id}`);

  const { data, error } = await supabase.storage
    .from(BUCKET)
    .createSignedUrl(row.storage_path, SIGNED_URL_TTL);
  if (error || !data) throw new Error(`Could not sign audio URL: ${error?.message ?? 'unknown'}`);
  return data.signedUrl;
}
