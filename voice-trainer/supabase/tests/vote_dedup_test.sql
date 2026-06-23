-- Strict one-vote-per-pair + undo tests (npx supabase test db)

begin;
select plan(5);

insert into auth.users (instance_id, id, aud, role, email, created_at, updated_at,
                        raw_app_meta_data, raw_user_meta_data, is_anonymous)
values
  ('00000000-0000-0000-0000-000000000000', 'a0000000-0000-0000-0000-000000000001', 'authenticated', 'authenticated', 'a@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'b0000000-0000-0000-0000-000000000002', 'authenticated', 'authenticated', 'b@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'd0000000-0000-0000-0000-000000000004', 'authenticated', 'authenticated', 'c@test.dev', now(), now(), '{}', '{}', false);

insert into public.recordings (id, user_id, name, storage_path, size_bytes, visibility) values
  ('c0000000-0000-0000-0000-0000000000a1', 'a0000000-0000-0000-0000-000000000001', 'p1', 'a0000000-0000-0000-0000-000000000001/1.webm', 100, 'public'),
  ('c0000000-0000-0000-0000-0000000000a2', 'a0000000-0000-0000-0000-000000000001', 'p2', 'a0000000-0000-0000-0000-000000000001/2.webm', 100, 'public');

insert into public.comparison_sets (id, creator_id, name)
values ('5e700000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'S');
insert into public.comparison_items (set_id, recording_id) values
  ('5e700000-0000-0000-0000-000000000001', 'c0000000-0000-0000-0000-0000000000a1'),
  ('5e700000-0000-0000-0000-000000000001', 'c0000000-0000-0000-0000-0000000000a2');

-- ── B votes on the pair (a1 vs a2) ──────────────────────────────────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select lives_ok(
  $$ insert into public.comparison_votes (id, set_id, recording_a, recording_b, winner_id, voter_id)
     values ('f0000000-0000-0000-0000-0000000000b1','5e700000-0000-0000-0000-000000000001',
             'c0000000-0000-0000-0000-0000000000a1','c0000000-0000-0000-0000-0000000000a2',
             'c0000000-0000-0000-0000-0000000000a1','b0000000-0000-0000-0000-000000000002') $$,
  'B can vote on the pair once');

-- Same pair, REVERSED order → still blocked (unordered uniqueness).
select throws_ok(
  $$ insert into public.comparison_votes (set_id, recording_a, recording_b, winner_id, voter_id)
     values ('5e700000-0000-0000-0000-000000000001',
             'c0000000-0000-0000-0000-0000000000a2','c0000000-0000-0000-0000-0000000000a1',
             'c0000000-0000-0000-0000-0000000000a2','b0000000-0000-0000-0000-000000000002') $$,
  '23505', null,
  'B cannot vote on the same pair again (even reversed)');

-- ── C cannot delete B's vote ────────────────────────────────────────────────
reset role;
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"d0000000-0000-0000-0000-000000000004","role":"authenticated"}';
delete from public.comparison_votes where id = 'f0000000-0000-0000-0000-0000000000b1';
reset role;
select is(
  (select count(*) from public.comparison_votes where id = 'f0000000-0000-0000-0000-0000000000b1')::int,
  1, 'C cannot delete B''s vote');

-- ── B can undo (delete) their own vote, then re-vote the pair ───────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';
delete from public.comparison_votes where id = 'f0000000-0000-0000-0000-0000000000b1';
reset role;
select is(
  (select count(*) from public.comparison_votes where voter_id = 'b0000000-0000-0000-0000-000000000002')::int,
  0, 'B can undo their own vote');

set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';
select lives_ok(
  $$ insert into public.comparison_votes (set_id, recording_a, recording_b, winner_id, voter_id)
     values ('5e700000-0000-0000-0000-000000000001',
             'c0000000-0000-0000-0000-0000000000a1','c0000000-0000-0000-0000-0000000000a2',
             'c0000000-0000-0000-0000-0000000000a2','b0000000-0000-0000-0000-000000000002') $$,
  'after undo, B can vote on the pair again');

select * from finish();
rollback;
