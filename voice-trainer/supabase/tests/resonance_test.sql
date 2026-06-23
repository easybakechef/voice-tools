-- Resonance community: visibility, blind voting, stats (npx supabase test db)

begin;
select plan(6);

insert into auth.users (instance_id, id, aud, role, email, created_at, updated_at,
                        raw_app_meta_data, raw_user_meta_data, is_anonymous)
values
  ('00000000-0000-0000-0000-000000000000', 'a0000000-0000-0000-0000-000000000001', 'authenticated', 'authenticated', 'a@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'b0000000-0000-0000-0000-000000000002', 'authenticated', 'authenticated', 'b@test.dev', now(), now(), '{}', '{}', false);

insert into public.sample_phrases (id, text, sort) values ('11110000-0000-0000-0000-000000000001', 'Test.', 99);

-- A owns a pair (private by default) with a deep + bright sample.
insert into public.dataset_pairs (id, speaker_id, phrase_id)
values ('da7a0000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', '11110000-0000-0000-0000-000000000001');
insert into public.dataset_samples (id, pair_id, speaker_id, label, storage_path) values
  ('5a330000-0000-0000-0000-0000000000de', 'da7a0000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'deep',   'a/deep.webm'),
  ('5a330000-0000-0000-0000-0000000000b7', 'da7a0000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'bright', 'a/bright.webm');

-- ── B cannot vote while the pair is private ─────────────────────────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select is(
  (select count(*) from public.dataset_pairs where id = 'da7a0000-0000-0000-0000-000000000001')::int,
  0, 'B cannot see A''s PRIVATE pair');

select throws_ok(
  $$ insert into public.resonance_votes (pair_id, voter_id, chosen_sample_id)
     values ('da7a0000-0000-0000-0000-000000000001','b0000000-0000-0000-0000-000000000002','5a330000-0000-0000-0000-0000000000b7') $$,
  '42501', null,
  'B cannot vote on a private pair');

-- ── A publishes the pair ────────────────────────────────────────────────────
reset role;
update public.dataset_pairs set visibility = 'public' where id = 'da7a0000-0000-0000-0000-000000000001';

set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select is(
  (select count(*) from public.dataset_samples where pair_id = 'da7a0000-0000-0000-0000-000000000001')::int,
  2, 'B can now see both samples of the public pair');

select lives_ok(
  $$ insert into public.resonance_votes (pair_id, voter_id, chosen_sample_id)
     values ('da7a0000-0000-0000-0000-000000000001','b0000000-0000-0000-0000-000000000002','5a330000-0000-0000-0000-0000000000b7') $$,
  'B can vote on the public pair (picked the bright take)');

select throws_ok(
  $$ insert into public.resonance_votes (pair_id, voter_id, chosen_sample_id)
     values ('da7a0000-0000-0000-0000-000000000001','b0000000-0000-0000-0000-000000000002','5a330000-0000-0000-0000-0000000000de') $$,
  '23505', null,
  'B cannot vote twice on the same pair');

-- ── Stats reflect the vote (1 for bright, 0 for deep) ───────────────────────
reset role;
select is(
  (select votes from public.resonance_pair_stats('da7a0000-0000-0000-0000-000000000001') where label = 'bright')::int,
  1, 'stats show 1 vote for the bright take');

select * from finish();
rollback;
