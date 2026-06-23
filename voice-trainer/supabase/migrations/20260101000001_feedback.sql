-- ============================================================================
--  Feedback: comments on recordings + audio access for shared recordings
--
--  Privacy model:
--    • Recordings stay PRIVATE by default. An owner opts in to feedback by
--      flipping a recording to `visibility = 'public'`.
--    • Anyone signed in can read a public recording, play its audio, and leave
--      a comment. The owner can always read comments on their own recording
--      (even after flipping it back to private).
-- ============================================================================

create table public.comments (
  id            uuid primary key default gen_random_uuid(),
  recording_id  uuid not null references public.recordings(id) on delete cascade,
  author_id     uuid not null references auth.users(id) on delete cascade,
  body          text not null check (char_length(body) between 1 and 2000),
  created_at    timestamptz not null default now()
);

create index comments_recording_idx on public.comments (recording_id, created_at);

alter table public.comments enable row level security;

-- Read a comment if: you wrote it, OR you can see the parent recording
-- (it's public, or you own it). The subqueries run under recordings' own RLS.
create policy "read comments on visible recordings"
  on public.comments for select
  using (
    author_id = auth.uid()
    or exists (
      select 1 from public.recordings r
      where r.id = comments.recording_id
        and (r.visibility = 'public' or r.user_id = auth.uid())
    )
  );

-- Comment only as yourself, and only on a recording that is currently public.
create policy "comment on public recordings"
  on public.comments for insert
  with check (
    author_id = auth.uid()
    and exists (
      select 1 from public.recordings r
      where r.id = comments.recording_id and r.visibility = 'public'
    )
  );

-- Delete your own comment, or moderate comments on a recording you own.
create policy "delete own comments or moderate own recording"
  on public.comments for delete
  using (
    author_id = auth.uid()
    or exists (
      select 1 from public.recordings r
      where r.id = comments.recording_id and r.user_id = auth.uid()
    )
  );

grant select, insert, delete on public.comments to authenticated, service_role;

-- ── Storage: let others read the audio of PUBLIC recordings ──────────────────
-- The init migration restricted audio to its owner. This adds read access to
-- objects whose recording row is public, so signed URLs work on the community
-- feed. Private recordings remain owner-only.
create policy "read audio of public recordings"
  on storage.objects for select
  using (
    bucket_id = 'recordings'
    and exists (
      select 1 from public.recordings r
      where r.storage_path = storage.objects.name
        and r.visibility = 'public'
    )
  );
