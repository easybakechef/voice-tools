-- ============================================================================
--  Let users edit their own display_name (for the account page) — but ONLY
--  display_name. tier / storage_limit_bytes / bytes_used / approved stay
--  admin-only so a user can't escalate their own quota or tier.
-- ============================================================================

create policy "update own profile"
  on public.profiles for update
  using (id = auth.uid())
  with check (id = auth.uid());

-- Column-scoped privilege: the RLS policy permits updating your own row, but
-- this grant means the only column an end user may actually write is
-- display_name. Writing any other column → permission denied.
grant update (display_name) on public.profiles to authenticated;
