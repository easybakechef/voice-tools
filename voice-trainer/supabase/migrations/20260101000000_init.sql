-- ============================================================================
--  Voice Trainer — initial schema
--
--  Migrated from the Rust/SQLite backend. Mirrors the old `recordings` table
--  but adds:
--    • per-user ownership (auth.users) + RLS isolation
--    • a private Storage bucket for audio (replaces on-disk files)
--    • a `profiles` table for quota / tier / waitlist (foundation for billing)
--    • a per-user byte quota enforced in the database
--
--  Design notes for future features (comments, ratings, waitlist enforcement):
--    - `recordings.visibility` already gates public sharing.
--    - `profiles.approved` / `profiles.tier` are present but NOT yet enforced on
--      upload, so the app keeps its current "anyone can record & save" behavior.
--      Flip the commented-out clause in the INSERT policy to turn the waitlist on.
-- ============================================================================

-- ── profiles : app-owned account data, 1:1 with auth.users ──────────────────
create table public.profiles (
  id                  uuid primary key references auth.users(id) on delete cascade,
  display_name        text,
  approved            boolean not null default true,          -- waitlist gate (default-open for now)
  tier                text    not null default 'free',        -- drives storage_limit_bytes
  storage_limit_bytes bigint  not null default 52428800,      -- 50 MB free tier
  bytes_used          bigint  not null default 0,             -- maintained by trigger
  created_at          timestamptz not null default now()
);

alter table public.profiles enable row level security;

-- Users may read ONLY their own profile. There is deliberately no INSERT/UPDATE
-- policy: profile rows are created by a trigger, and tier/quota/approved are
-- admin-only (changed via the service_role key), so users can't escalate quota.
create policy "read own profile"
  on public.profiles for select
  using (auth.uid() = id);

-- Auto-create a profile whenever a new auth user (incl. anonymous) signs up.
create function public.handle_new_user()
returns trigger
language plpgsql
security definer set search_path = ''
as $$
begin
  insert into public.profiles (id) values (new.id)
  on conflict (id) do nothing;
  return new;
end;
$$;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();

-- ── recordings : the migrated core table ────────────────────────────────────
create table public.recordings (
  id            uuid primary key default gen_random_uuid(),
  user_id       uuid not null references auth.users(id) on delete cascade,
  name          text not null default 'Untitled',
  recorded_at   timestamptz not null default now(),   -- was `date` (epoch ms)
  duration      double precision not null default 0,
  median_pitch  double precision not null default 0,
  storage_path  text not null,                        -- object key in the bucket
  size_bytes    bigint not null default 0,
  visibility    text not null default 'private'
                  check (visibility in ('private', 'public')),
  pitch_log     jsonb not null default '[]'::jsonb,
  formant_data  jsonb not null default '[]'::jsonb,
  stats         jsonb,
  created_at    timestamptz not null default now()
);

create index recordings_user_id_idx     on public.recordings (user_id, recorded_at desc);
create index recordings_public_idx      on public.recordings (visibility) where visibility = 'public';

alter table public.recordings enable row level security;

-- Read: your own recordings, plus any explicitly shared for review.
create policy "read own or shared recordings"
  on public.recordings for select
  using (auth.uid() = user_id or visibility = 'public');

-- Insert: only as yourself.
-- To turn on the waitlist later, add:  and (select approved from public.profiles where id = auth.uid())
create policy "insert own recordings"
  on public.recordings for insert
  with check (auth.uid() = user_id);

-- Update: only your own rows, and you can't reassign ownership.
create policy "update own recordings"
  on public.recordings for update
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

-- Delete: only your own.
create policy "delete own recordings"
  on public.recordings for delete
  using (auth.uid() = user_id);

grant select, insert, update, delete on public.recordings to authenticated, service_role;
grant select on public.profiles to authenticated;
grant select, insert, update, delete on public.profiles to service_role;

-- ── per-user byte quota ─────────────────────────────────────────────────────
-- Enforced before insert; the running total lives on profiles.bytes_used.
create function public.check_recording_quota()
returns trigger
language plpgsql
security definer set search_path = ''
as $$
declare
  used  bigint;
  limit_bytes bigint;
begin
  select bytes_used, storage_limit_bytes
    into used, limit_bytes
    from public.profiles
   where id = new.user_id
   for update;

  if used + new.size_bytes > limit_bytes then
    raise exception 'storage quota exceeded: % + % > %', used, new.size_bytes, limit_bytes
      using errcode = 'check_violation';
  end if;
  return new;
end;
$$;

create trigger recordings_check_quota
  before insert on public.recordings
  for each row execute function public.check_recording_quota();

-- Keep profiles.bytes_used in sync as recordings come and go.
create function public.track_recording_usage()
returns trigger
language plpgsql
security definer set search_path = ''
as $$
begin
  if tg_op = 'INSERT' then
    update public.profiles set bytes_used = bytes_used + new.size_bytes where id = new.user_id;
  elsif tg_op = 'DELETE' then
    update public.profiles set bytes_used = greatest(0, bytes_used - old.size_bytes) where id = old.user_id;
  end if;
  return null;
end;
$$;

create trigger recordings_track_usage
  after insert or delete on public.recordings
  for each row execute function public.track_recording_usage();

-- ── private Storage bucket for audio blobs ──────────────────────────────────
-- 'audio/*' wildcard so codec-qualified types (e.g. 'audio/webm;codecs=opus'
-- from MediaRecorder) and uploaded files of any audio format are accepted.
insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values ('recordings', 'recordings', false, 26214400, array['audio/*'])
on conflict (id) do nothing;

-- Files are namespaced by owner: object name = '<user_id>/<uuid>.webm'.
-- Access is owner-only for every operation. (Playback of *shared* recordings is
-- a future feature: serve those via short-lived signed URLs minted server-side
-- after a visibility check, or add a policy keyed on a public recordings row.)
create policy "owner reads own audio"
  on storage.objects for select
  using (bucket_id = 'recordings' and (storage.foldername(name))[1] = auth.uid()::text);

create policy "owner uploads own audio"
  on storage.objects for insert
  with check (bucket_id = 'recordings' and (storage.foldername(name))[1] = auth.uid()::text);

create policy "owner updates own audio"
  on storage.objects for update
  using (bucket_id = 'recordings' and (storage.foldername(name))[1] = auth.uid()::text);

create policy "owner deletes own audio"
  on storage.objects for delete
  using (bucket_id = 'recordings' and (storage.foldername(name))[1] = auth.uid()::text);
