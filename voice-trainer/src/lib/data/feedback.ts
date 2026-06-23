// ============================================================================
//  Feedback (comments) data-access layer
//
//  Like recordings.ts, this is the only place the app talks to Supabase for
//  comments. Components import these functions, never the Supabase client.
// ============================================================================

import { supabase, currentUserId } from '$lib/supabase/client.js';

export interface FeedbackComment {
  id: string;
  recordingId: string;
  authorId: string;
  body: string;
  date: number; // epoch ms
  mine: boolean; // true if the current user wrote it
}

interface CommentRow {
  id: string;
  recording_id: string;
  author_id: string;
  body: string;
  created_at: string;
}

const COMMENT_COLUMNS = 'id, recording_id, author_id, body, created_at';

function rowToComment(r: CommentRow, myId: string): FeedbackComment {
  return {
    id: r.id,
    recordingId: r.recording_id,
    authorId: r.author_id,
    body: r.body,
    date: Date.parse(r.created_at),
    mine: r.author_id === myId,
  };
}

/** Comments on a recording, oldest first. RLS decides what's visible. */
export async function listComments(recordingId: string): Promise<FeedbackComment[]> {
  const myId = await currentUserId();
  const { data, error } = await supabase
    .from('comments')
    .select(COMMENT_COLUMNS)
    .eq('recording_id', recordingId)
    .order('created_at', { ascending: true });

  if (error) throw new Error(`Failed to load feedback: ${error.message}`);
  return (data as CommentRow[]).map((r) => rowToComment(r, myId));
}

/** Leave feedback on a recording. Only permitted on public recordings (RLS). */
export async function addComment(recordingId: string, body: string): Promise<FeedbackComment> {
  const myId = await currentUserId();
  const trimmed = body.trim();
  if (!trimmed) throw new Error('Comment is empty');

  const { data, error } = await supabase
    .from('comments')
    .insert({ recording_id: recordingId, author_id: myId, body: trimmed })
    .select(COMMENT_COLUMNS)
    .single();

  if (error || !data) throw new Error(`Could not post feedback: ${error?.message ?? 'unknown'}`);
  return rowToComment(data as CommentRow, myId);
}

/** Delete a comment (author, or owner of the recording, per RLS). */
export async function deleteComment(id: string): Promise<void> {
  const { error } = await supabase.from('comments').delete().eq('id', id);
  if (error) throw new Error(`Could not delete comment: ${error.message}`);
}

/** Short, stable pseudonym for an anonymous author (UUIDs aren't friendly). */
export function authorLabel(c: FeedbackComment): string {
  if (c.mine) return 'You';
  return `Anon ${c.authorId.slice(0, 6)}`;
}
