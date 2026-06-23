-- ============================================================================
--  Pairwise comparison / ranking
--
--  A "comparison set" is a named collection of PUBLIC recordings. Voters are
--  shown two clips from the same set and pick which is "more <attribute>"
--  (default: feminine). Votes accumulate into win-rates (rank_set()), the
--  foundation for a femininity-scoring model (Bradley-Terry / Elo later).
-- ============================================================================

-- ── comparison_sets ─────────────────────────────────────────────────────────
create table public.comparison_sets (
  id          uuid primary key default gen_random_uuid(),
  creator_id  uuid not null references auth.users(id) on delete cascade,
  name        text not null check (char_length(name) between 1 and 120),
  description text,
  created_at  timestamptz not null default now()
);

alter table public.comparison_sets enable row level security;

-- Sets are public challenges: any signed-in user can see them and vote.
create policy "read all sets"        on public.comparison_sets for select using (true);
create policy "create own set"       on public.comparison_sets for insert with check (creator_id = auth.uid());
create policy "update own set"       on public.comparison_sets for update using (creator_id = auth.uid()) with check (creator_id = auth.uid());
create policy "delete own set"       on public.comparison_sets for delete using (creator_id = auth.uid());

grant select, insert, update, delete on public.comparison_sets to authenticated, service_role;

-- ── comparison_items : which recordings belong to a set ─────────────────────
create table public.comparison_items (
  id           uuid primary key default gen_random_uuid(),
  set_id       uuid not null references public.comparison_sets(id) on delete cascade,
  recording_id uuid not null references public.recordings(id) on delete cascade,
  added_at     timestamptz not null default now(),
  unique (set_id, recording_id)
);
create index comparison_items_set_idx on public.comparison_items (set_id);

alter table public.comparison_items enable row level security;

create policy "read all items" on public.comparison_items for select using (true);

-- You may add an item only to a set YOU own, and only if the recording is
-- public (so other voters can actually see and play it).
create policy "add items to own set"
  on public.comparison_items for insert
  with check (
    exists (select 1 from public.comparison_sets s where s.id = set_id and s.creator_id = auth.uid())
    and exists (select 1 from public.recordings r where r.id = recording_id and r.visibility = 'public')
  );

create policy "remove items from own set"
  on public.comparison_items for delete
  using (exists (select 1 from public.comparison_sets s where s.id = set_id and s.creator_id = auth.uid()));

grant select, insert, delete on public.comparison_items to authenticated, service_role;

-- ── comparison_votes : one pairwise judgment ────────────────────────────────
create table public.comparison_votes (
  id           uuid primary key default gen_random_uuid(),
  set_id       uuid not null references public.comparison_sets(id) on delete cascade,
  attribute    text not null default 'feminine',
  recording_a  uuid not null references public.recordings(id) on delete cascade,
  recording_b  uuid not null references public.recordings(id) on delete cascade,
  winner_id    uuid not null references public.recordings(id) on delete cascade,
  voter_id     uuid not null references auth.users(id) on delete cascade,
  created_at   timestamptz not null default now(),
  check (recording_a <> recording_b),
  check (winner_id = recording_a or winner_id = recording_b)
);
create index comparison_votes_set_idx on public.comparison_votes (set_id);

alter table public.comparison_votes enable row level security;

-- Cast votes as yourself. Individual votes are readable only by the voter and
-- the set's creator (aggregates are exposed separately via rank_set()).
create policy "cast own vote"
  on public.comparison_votes for insert
  with check (voter_id = auth.uid());

create policy "read own votes or as set creator"
  on public.comparison_votes for select
  using (
    voter_id = auth.uid()
    or exists (select 1 from public.comparison_sets s where s.id = set_id and s.creator_id = auth.uid())
  );

grant select, insert on public.comparison_votes to authenticated;
grant select, insert, update, delete on public.comparison_votes to service_role;

-- ── aggregate ranking for a set ─────────────────────────────────────────────
-- SECURITY DEFINER so it can tally ALL votes regardless of the caller, but it
-- only returns anonymous aggregates (no voter identities) for PUBLIC recordings.
create function public.rank_set(p_set_id uuid)
returns table (recording_id uuid, name text, wins bigint, comparisons bigint, win_rate numeric)
language sql
stable
security definer
set search_path = ''
as $$
  with items as (
    select i.recording_id
    from public.comparison_items i
    where i.set_id = p_set_id
  ),
  v as (
    select * from public.comparison_votes where set_id = p_set_id
  )
  select
    r.id,
    r.name,
    (select count(*) from v where v.winner_id = r.id) as wins,
    (select count(*) from v where v.recording_a = r.id or v.recording_b = r.id) as comparisons,
    case
      when (select count(*) from v where v.recording_a = r.id or v.recording_b = r.id) > 0
      then round(
        (select count(*) from v where v.winner_id = r.id)::numeric
        / (select count(*) from v where v.recording_a = r.id or v.recording_b = r.id), 3)
      else 0
    end as win_rate
  from items
  join public.recordings r on r.id = items.recording_id and r.visibility = 'public'
  order by win_rate desc, comparisons desc;
$$;

grant execute on function public.rank_set(uuid) to authenticated, service_role;
