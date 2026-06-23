-- Dataset pair/sample access-control tests (npx supabase test db)

begin;
select plan(5);

insert into auth.users (instance_id, id, aud, role, email, created_at, updated_at,
                        raw_app_meta_data, raw_user_meta_data, is_anonymous)
values
  ('00000000-0000-0000-0000-000000000000', 'a0000000-0000-0000-0000-000000000001', 'authenticated', 'authenticated', 'a@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'b0000000-0000-0000-0000-000000000002', 'authenticated', 'authenticated', 'b@test.dev', now(), now(), '{}', '{}', false);

-- Phrases are seeded by the migration; grab one.
-- (Use a fixed phrase via insert so the test is independent of seed contents.)
insert into public.sample_phrases (id, text, sort)
values ('11110000-0000-0000-0000-000000000001', 'Test phrase.', 99);

-- ── A creates a pair with two samples ───────────────────────────────────────
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"a0000000-0000-0000-0000-000000000001","role":"authenticated"}';

select lives_ok(
  $$ insert into public.dataset_pairs (id, speaker_id, phrase_id)
     values ('da7a0000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001','11110000-0000-0000-0000-000000000001') $$,
  'A can create a pair');

select lives_ok(
  $$ insert into public.dataset_samples (pair_id, speaker_id, label, storage_path) values
     ('da7a0000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001','deep',  'a0000000-0000-0000-0000-000000000001/dataset/da7a0000-deep-resonance.webm'),
     ('da7a0000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001','bright','a0000000-0000-0000-0000-000000000001/dataset/da7a0000-bright-resonance.webm') $$,
  'A can add deep + bright samples');

-- one label per pair
select throws_ok(
  $$ insert into public.dataset_samples (pair_id, speaker_id, label, storage_path)
     values ('da7a0000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001','deep','x.webm') $$,
  '23505', null,
  'a pair cannot have two samples with the same label');

-- ── B cannot see A's dataset ────────────────────────────────────────────────
reset role;
set local role authenticated;
set local "request.jwt.claims" to '{"sub":"b0000000-0000-0000-0000-000000000002","role":"authenticated"}';

select is(
  (select count(*) from public.dataset_pairs where speaker_id = 'a0000000-0000-0000-0000-000000000001')::int,
  0, 'B cannot read A''s pairs');

select is(
  (select count(*) from public.dataset_samples where pair_id = 'da7a0000-0000-0000-0000-000000000001')::int,
  0, 'B cannot read A''s samples');

select * from finish();
rollback;
