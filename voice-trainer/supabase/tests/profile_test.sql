-- Profile self-edit access-control tests (npx supabase test db)

begin;
select plan(4);

insert into auth.users (instance_id, id, aud, role, email, created_at, updated_at,
                        raw_app_meta_data, raw_user_meta_data, is_anonymous)
values
  ('00000000-0000-0000-0000-000000000000', 'a0000000-0000-0000-0000-000000000001', 'authenticated', 'authenticated', 'a@test.dev', now(), now(), '{}', '{}', false),
  ('00000000-0000-0000-0000-000000000000', 'b0000000-0000-0000-0000-000000000002', 'authenticated', 'authenticated', 'b@test.dev', now(), now(), '{}', '{}', false);
-- profiles auto-created by the on_auth_user_created trigger

set local role authenticated;
set local "request.jwt.claims" to '{"sub":"a0000000-0000-0000-0000-000000000001","role":"authenticated"}';

-- ✓ can rename yourself
select lives_ok(
  $$ update public.profiles set display_name = 'Mara' where id = 'a0000000-0000-0000-0000-000000000001' $$,
  'user can update their own display_name');

select is(
  (select display_name from public.profiles where id = 'a0000000-0000-0000-0000-000000000001'),
  'Mara', 'display_name was changed');

-- ✗ cannot escalate your own tier/quota (no column privilege)
select throws_ok(
  $$ update public.profiles set tier = 'premium' where id = 'a0000000-0000-0000-0000-000000000001' $$,
  '42501', null,
  'user cannot change their own tier');

-- ✗ cannot rename someone else (RLS row policy → 0 rows)
with upd as (
  update public.profiles set display_name = 'hacked' where id = 'b0000000-0000-0000-0000-000000000002' returning 1
)
select is((select count(*) from upd)::int, 0, 'user cannot edit another profile');

select * from finish();
rollback;
