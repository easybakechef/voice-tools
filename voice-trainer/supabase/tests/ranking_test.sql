-- Comparison / ranking access-control + aggregate tests (npx supabase test db)

begin;
select plan(6);

-- Users: A = set creator/owner of recordings; B = voter.
insert into auth.users (instance_id, id, aud, role, email, created_at, updated_at,
                        raw_app_meta_data, raw_user_meta_data, is_anonymous)
values
  ('00000000-0000-0000-0000-000000000000', 'a0000000-0000-0000-0000-000000000001',
   'authenticated', 'authenticated', 'a@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'b0000000-0000-0000-0000-000000000002',
   'authenticated', 'authenticated', 'b@test.dev', now(), now(), '{}', '{}', false);

-- Two PUBLIC recordings + one PRIVATE one, all owned by A.
insert into public.recordings (id, user_id, name, storage_path, size_bytes, visibility) values
  ('c0000000-0000-0000-0000-0000000000a1', 'a0000000-0000-0000-0000-000000000001', 'pub 1', 'a0000000-0000-0000-0000-000000000001/1.webm', 100, 'public'),
  ('c0000000-0000-0000-0000-0000000000a2', 'a0000000-0000-0000-0000-000000000001', 'pub 2', 'a0000000-0000-0000-0000-000000000001/2.webm', 100, 'public'),
  ('c0000000-0000-0000-0000-0000000000a3', 'a0000000-0000-0000-0000-000000000001', 'priv',  'a0000000-0000-0000-0000-000000000001/3.webm', 100, 'private');

-- ── A creates a set ─────────────────────────────────────────────────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"a0000000-0000-0000-0000-000000000001","role":"authenticated"}';

insert into public.comparison_sets (id, creator_id, name)
values ('5e700000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'Dataset A');

select lives_ok(
  $$ insert into public.comparison_items (set_id, recording_id) values
     ('5e700000-0000-0000-0000-000000000001','c0000000-0000-0000-0000-0000000000a1'),
     ('5e700000-0000-0000-0000-000000000001','c0000000-0000-0000-0000-0000000000a2') $$,
  'owner can add PUBLIC recordings to their set');

select throws_ok(
  $$ insert into public.comparison_items (set_id, recording_id)
     values ('5e700000-0000-0000-0000-000000000001','c0000000-0000-0000-0000-0000000000a3') $$,
  '42501', null,
  'cannot add a PRIVATE recording to a set');

-- ── B cannot add items to A's set ───────────────────────────────────────────
reset role;
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select throws_ok(
  $$ insert into public.comparison_items (set_id, recording_id)
     values ('5e700000-0000-0000-0000-000000000001','c0000000-0000-0000-0000-0000000000a1') $$,
  '42501', null,
  'non-owner cannot add items to someone else''s set');

-- ── B votes ─────────────────────────────────────────────────────────────────
select lives_ok(
  $$ insert into public.comparison_votes (set_id, recording_a, recording_b, winner_id, voter_id)
     values ('5e700000-0000-0000-0000-000000000001',
             'c0000000-0000-0000-0000-0000000000a1','c0000000-0000-0000-0000-0000000000a2',
             'c0000000-0000-0000-0000-0000000000a1','b0000000-0000-0000-0000-000000000002') $$,
  'B can cast a vote as themselves');

select throws_ok(
  $$ insert into public.comparison_votes (set_id, recording_a, recording_b, winner_id, voter_id)
     values ('5e700000-0000-0000-0000-000000000001',
             'c0000000-0000-0000-0000-0000000000a1','c0000000-0000-0000-0000-0000000000a2',
             'c0000000-0000-0000-0000-0000000000a1','a0000000-0000-0000-0000-000000000001') $$,
  '42501', null,
  'B cannot cast a vote attributed to A');

-- ── aggregate reflects the vote (a1 beat a2 once) ───────────────────────────
reset role;
select is(
  (select wins from public.rank_set('5e700000-0000-0000-0000-000000000001')
    where recording_id = 'c0000000-0000-0000-0000-0000000000a1')::int,
  1, 'rank_set tallies a1 with 1 win');

select * from finish();
rollback;
