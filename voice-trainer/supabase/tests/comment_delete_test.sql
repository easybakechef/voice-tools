-- Comment deletion / moderation access-control tests (npx supabase test db)

begin;
select plan(4);

-- Three users: A owns the recording; B and C are commenters.
insert into auth.users (instance_id, id, aud, role, email, created_at, updated_at,
                        raw_app_meta_data, raw_user_meta_data, is_anonymous)
values
  ('00000000-0000-0000-0000-000000000000', 'a0000000-0000-0000-0000-000000000001',
   'authenticated', 'authenticated', 'a@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'b0000000-0000-0000-0000-000000000002',
   'authenticated', 'authenticated', 'b@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'd0000000-0000-0000-0000-000000000004',
   'authenticated', 'authenticated', 'c@test.dev', now(), now(), '{}', '{}', false);

-- A owns a PUBLIC recording with two comments: one by B, one by C.
insert into public.recordings (id, user_id, name, storage_path, size_bytes, visibility)
values ('c0000000-0000-0000-0000-000000000003',
        'a0000000-0000-0000-0000-000000000001',
        'A clip', 'a0000000-0000-0000-0000-000000000001/clip.webm', 100, 'public');

insert into public.comments (id, recording_id, author_id, body) values
  ('e0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-000000000003', 'b0000000-0000-0000-0000-000000000002', 'comment by B'),
  ('e0000000-0000-0000-0000-0000000000c1', 'c0000000-0000-0000-0000-000000000003', 'd0000000-0000-0000-0000-000000000004', 'comment by C');

-- ── C cannot delete B's comment (not the author, not the recording owner) ────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"d0000000-0000-0000-0000-000000000004","role":"authenticated"}';
delete from public.comments where id = 'e0000000-0000-0000-0000-0000000000b1';
reset role;
select is(
  (select count(*) from public.comments where id = 'e0000000-0000-0000-0000-0000000000b1')::int,
  1, 'C cannot delete B''s comment (RLS blocks it — 0 rows affected)');

-- ── B can delete their OWN comment ──────────────────────────────────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';
delete from public.comments where id = 'e0000000-0000-0000-0000-0000000000b1';
reset role;
select is(
  (select count(*) from public.comments where id = 'e0000000-0000-0000-0000-0000000000b1')::int,
  0, 'B can delete their own comment');

-- ── A (recording owner) can delete C's comment on A's recording ─────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"a0000000-0000-0000-0000-000000000001","role":"authenticated"}';
delete from public.comments where id = 'e0000000-0000-0000-0000-0000000000c1';
reset role;
select is(
  (select count(*) from public.comments where id = 'e0000000-0000-0000-0000-0000000000c1')::int,
  0, 'A (owner) can moderate/delete C''s comment on A''s recording');

-- ── Sanity: no comments left on the recording ───────────────────────────────
select is(
  (select count(*) from public.comments where recording_id = 'c0000000-0000-0000-0000-000000000003')::int,
  0, 'all comments removed');

select * from finish();
rollback;
