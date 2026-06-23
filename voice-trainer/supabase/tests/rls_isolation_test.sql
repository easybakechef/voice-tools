-- Cross-user data isolation tests (run with: npx supabase test db)
--
-- These prove the privacy guarantee that you can't verify by hand while logged
-- in as yourself: that user B cannot reach user A's private data through RLS.

begin;
select plan(5);

-- ── two test users (profiles auto-created by the on_auth_user_created trigger) ──
insert into auth.users (instance_id, id, aud, role, email, created_at, updated_at,
                        raw_app_meta_data, raw_user_meta_data, is_anonymous)
values
  ('00000000-0000-0000-0000-000000000000', 'a0000000-0000-0000-0000-000000000001',
   'authenticated', 'authenticated', 'a@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'b0000000-0000-0000-0000-000000000002',
   'authenticated', 'authenticated', 'b@test.dev', now(), now(), '{}', '{}', false);

select is(
  (select count(*) from public.profiles where id = 'a0000000-0000-0000-0000-000000000001')::int,
  1, 'profile auto-created for new user A');

-- User A owns one PRIVATE recording (inserted here as superuser).
insert into public.recordings (id, user_id, name, storage_path, size_bytes, visibility)
values ('c0000000-0000-0000-0000-000000000003',
        'a0000000-0000-0000-0000-000000000001',
        'A private clip', 'a0000000-0000-0000-0000-000000000001/clip.webm', 100, 'private');

-- ── Act as user B ───────────────────────────────────────────────────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select is(
  (select count(*) from public.recordings
    where user_id = 'a0000000-0000-0000-0000-000000000001')::int,
  0, 'B cannot SELECT A''s PRIVATE recording');

select throws_ok(
  $$ insert into public.recordings (user_id, storage_path, size_bytes)
     values ('a0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001/x.webm', 1) $$,
  '42501', null,
  'B cannot INSERT a recording owned by A');

-- ── A shares the clip for review ────────────────────────────────────────────
reset role;
update public.recordings set visibility = 'public'
  where id = 'c0000000-0000-0000-0000-000000000003';

set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select is(
  (select count(*) from public.recordings
    where id = 'c0000000-0000-0000-0000-000000000003')::int,
  1, 'B CAN SELECT A''s recording once shared (visibility=public)');

-- ...but still cannot modify it (UPDATE policy is owner-only → 0 rows affected).
with upd as (
  update public.recordings set name = 'hacked'
   where id = 'c0000000-0000-0000-0000-000000000003'
  returning 1
)
select is((select count(*) from upd)::int, 0,
  'B cannot UPDATE A''s shared recording');

select * from finish();
rollback;
