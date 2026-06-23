-- Feedback / comments access-control tests (npx supabase test db)

begin;
select plan(5);

-- Two users: A (owner) and B (commenter).
insert into auth.users (instance_id, id, aud, role, email, created_at, updated_at,
                        raw_app_meta_data, raw_user_meta_data, is_anonymous)
values
  ('00000000-0000-0000-0000-000000000000', 'a0000000-0000-0000-0000-000000000001',
   'authenticated', 'authenticated', 'a@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'b0000000-0000-0000-0000-000000000002',
   'authenticated', 'authenticated', 'b@test.dev', now(), now(), '{}', '{}', false);

-- A owns a PRIVATE recording.
insert into public.recordings (id, user_id, name, storage_path, size_bytes, visibility)
values ('c0000000-0000-0000-0000-000000000003',
        'a0000000-0000-0000-0000-000000000001',
        'A clip', 'a0000000-0000-0000-0000-000000000001/clip.webm', 100, 'private');

-- ── B cannot comment while the recording is private ─────────────────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select throws_ok(
  $$ insert into public.comments (recording_id, author_id, body)
     values ('c0000000-0000-0000-0000-000000000003','b0000000-0000-0000-0000-000000000002','nice') $$,
  '42501', null,
  'B cannot comment on a PRIVATE recording');

-- ── A makes it public ───────────────────────────────────────────────────────
reset role;
update public.recordings set visibility = 'public'
  where id = 'c0000000-0000-0000-0000-000000000003';

-- ── B can now comment ───────────────────────────────────────────────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select lives_ok(
  $$ insert into public.comments (recording_id, author_id, body)
     values ('c0000000-0000-0000-0000-000000000003','b0000000-0000-0000-0000-000000000002','great resonance') $$,
  'B CAN comment once the recording is public');

-- B cannot post a comment authored by someone else.
select throws_ok(
  $$ insert into public.comments (recording_id, author_id, body)
     values ('c0000000-0000-0000-0000-000000000003','a0000000-0000-0000-0000-000000000001','spoof') $$,
  '42501', null,
  'B cannot post a comment as A');

-- ── A (owner) can read the feedback left on their recording ─────────────────
reset role;
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"a0000000-0000-0000-0000-000000000001","role":"authenticated"}';

select is(
  (select count(*) from public.comments
    where recording_id = 'c0000000-0000-0000-0000-000000000003')::int,
  1, 'A can read B''s feedback on A''s recording');

-- ── A flips it back to private; A still sees the feedback ───────────────────
reset role;
update public.recordings set visibility = 'private'
  where id = 'c0000000-0000-0000-0000-000000000003';

set local role authenticated;
set local "request.jwt.claims" to '{"sub":"a0000000-0000-0000-0000-000000000001","role":"authenticated"}';

select is(
  (select count(*) from public.comments
    where recording_id = 'c0000000-0000-0000-0000-000000000003')::int,
  1, 'owner still sees feedback after making the recording private again');

select * from finish();
rollback;
