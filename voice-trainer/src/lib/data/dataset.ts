// ============================================================================
//  Dataset (paired resonance recordings) data-access layer
//
//  Backend-agnostic interface. A "pair" is two takes of the same phrase — a
//  deep-resonance and a bright-resonance recording — sharing one uuid.
// ============================================================================

import { supabase, currentUserId } from '$lib/supabase/client.js';

const BUCKET = 'recordings';
const SIGNED_URL_TTL = 60 * 60;

export type ResonanceLabel = 'deep' | 'bright';

export interface SamplePhrase {
  id: string;
  text: string;
}

export type PairVisibility = 'private' | 'public';

export interface DatasetPair {
  id: string; // the shared uuid
  createdAt: number;
  phrase: string;
  visibility: PairVisibility;
  deepPath: string | null;
  brightPath: string | null;
}

/** Curated one-sentence phrases to read, in display order. */
export async function listPhrases(): Promise<SamplePhrase[]> {
  await currentUserId();
  const { data, error } = await supabase
    .from('sample_phrases')
    .select('id, text')
    .eq('active', true)
    .order('sort', { ascending: true });
  if (error) throw new Error(`Failed to load phrases: ${error.message}`);
  return data as SamplePhrase[];
}

function objectPath(speakerId: string, uuid: string, label: ResonanceLabel) {
  return `${speakerId}/dataset/${uuid}-${label}-resonance.webm`;
}

/**
 * Submit both takes of a phrase as one pair. Uploads the two audio files
 * (sharing `uuid` in their names) and indexes the pair + samples. Returns the
 * shared uuid. Cleans up partial uploads on failure.
 */
export async function submitPair(
  phraseId: string,
  deep: Blob,
  bright: Blob,
): Promise<string> {
  const speakerId = await currentUserId();
  const uuid = crypto.randomUUID();
  const deepPath = objectPath(speakerId, uuid, 'deep');
  const brightPath = objectPath(speakerId, uuid, 'bright');
  const uploaded: string[] = [];

  const upload = async (path: string, blob: Blob) => {
    const { error } = await supabase.storage
      .from(BUCKET)
      .upload(path, blob, { contentType: blob.type || 'audio/webm', upsert: false });
    if (error) throw new Error(`Audio upload failed: ${error.message}`);
    uploaded.push(path);
  };

  const cleanup = async () => {
    if (uploaded.length) await supabase.storage.from(BUCKET).remove(uploaded);
    await supabase.from('dataset_pairs').delete().eq('id', uuid);
  };

  try {
    await upload(deepPath, deep);
    await upload(brightPath, bright);

    const { error: pErr } = await supabase
      .from('dataset_pairs')
      .insert({ id: uuid, speaker_id: speakerId, phrase_id: phraseId });
    if (pErr) throw new Error(`Could not save pair: ${pErr.message}`);

    const { error: sErr } = await supabase.from('dataset_samples').insert([
      { pair_id: uuid, speaker_id: speakerId, label: 'deep', storage_path: deepPath },
      { pair_id: uuid, speaker_id: speakerId, label: 'bright', storage_path: brightPath },
    ]);
    if (sErr) throw new Error(`Could not index samples: ${sErr.message}`);

    return uuid;
  } catch (e) {
    await cleanup();
    throw e;
  }
}

/** The current user's recorded pairs, newest first. */
export async function listMyPairs(): Promise<DatasetPair[]> {
  await currentUserId();
  const { data, error } = await supabase
    .from('dataset_pairs')
    .select('id, created_at, visibility, phrase:sample_phrases(text), samples:dataset_samples(label, storage_path)')
    .order('created_at', { ascending: false });
  if (error) throw new Error(`Failed to load your dataset: ${error.message}`);

  return (data as any[]).map((p) => {
    const samples: { label: ResonanceLabel; storage_path: string }[] = p.samples ?? [];
    return {
      id: p.id,
      createdAt: Date.parse(p.created_at),
      visibility: p.visibility as PairVisibility,
      phrase: p.phrase?.text ?? '(phrase removed)',
      deepPath: samples.find((s) => s.label === 'deep')?.storage_path ?? null,
      brightPath: samples.find((s) => s.label === 'bright')?.storage_path ?? null,
    };
  });
}

/** Delete a pair and both of its audio files. */
export async function deletePair(id: string): Promise<void> {
  const { data, error: selErr } = await supabase
    .from('dataset_samples')
    .select('storage_path')
    .eq('pair_id', id);
  if (selErr) throw new Error(`Delete failed: ${selErr.message}`);

  const { error: delErr } = await supabase.from('dataset_pairs').delete().eq('id', id);
  if (delErr) throw new Error(`Delete failed: ${delErr.message}`);

  const paths = (data as { storage_path: string }[]).map((s) => s.storage_path);
  if (paths.length) await supabase.storage.from(BUCKET).remove(paths);
}

/** Publish a pair to the resonance community, or make it private again. */
export async function setPairVisibility(id: string, visibility: PairVisibility): Promise<void> {
  const { error } = await supabase.from('dataset_pairs').update({ visibility }).eq('id', id);
  if (error) throw new Error(`Could not update visibility: ${error.message}`);
}

/** Short-lived playable URL for a sample's audio (signed). */
export async function getSampleUrl(storagePath: string): Promise<string> {
  const { data, error } = await supabase.storage
    .from(BUCKET)
    .createSignedUrl(storagePath, SIGNED_URL_TTL);
  if (error || !data) throw new Error(`Could not sign audio URL: ${error?.message ?? 'unknown'}`);
  return data.signedUrl;
}
