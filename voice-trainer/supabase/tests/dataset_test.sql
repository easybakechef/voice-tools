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
  $$ insert into public.dataset_samples (id, pair_id, speaker_id, storage_path) values
     ('5a330000-0000-0000-0000-0000000000de','da7a0000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001/dataset/5a330000de.webm'),
     ('5a330000-0000-0000-0000-0000000000b7','da7a0000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001/dataset/5a330000b7.webm') $$,
  'A can add deep + bright samples (opaque paths, no label)');

select lives_ok(
  $$ insert into public.sample_labels (sample_id, pair_id, speaker_id, label) values
     ('5a330000-0000-0000-0000-0000000000de','da7a0000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001','deep'),
     ('5a330000-0000-0000-0000-0000000000b7','da7a0000-0000-0000-0000-000000000001','a0000000-0000-0000-0000-000000000001','bright') $$,
  'A can label them deep + bright');

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
