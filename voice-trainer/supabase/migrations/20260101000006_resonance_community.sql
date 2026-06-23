-- ============================================================================
--  Resonance community: publish paired recordings, blind "which is brighter"
--  voting with label reveal + stats, and comments on pairs.
-- ============================================================================

-- ── Make pairs publishable ──────────────────────────────────────────────────
alter table public.dataset_pairs
  add column visibility text not null default 'private'
    check (visibility in ('private', 'public'));
create index dataset_pairs_public_idx on public.dataset_pairs (visibility) where visibility = 'public';

-- Pairs: readable by owner OR anyone if public; owner can flip visibility.
drop policy "read own pairs" on public.dataset_pairs;
create policy "read own or public pairs"
  on public.dataset_pairs for select
  using (speaker_id = auth.uid() or visibility = 'public');
create policy "update own pairs"
  on public.dataset_pairs for update
  using (speaker_id = auth.uid()) with check (speaker_id = auth.uid());

-- Samples: readable by owner OR if their pair is public.
drop policy "read own samples" on public.dataset_samples;
create policy "read own or public samples"
  on public.dataset_samples for select
  using (
    speaker_id = auth.uid()
    or exists (select 1 from public.dataset_pairs p where p.id = pair_id and p.visibility = 'public')
  );

-- Storage: let others read audio belonging to a public pair.
create policy "read audio of public dataset pairs"
  on storage.objects for select
  using (
    bucket_id = 'recordings'
    and exists (
      select 1 from public.dataset_samples ds
      join public.dataset_pairs dp on dp.id = ds.pair_id
      where ds.storage_path = storage.objects.name and dp.visibility = 'public'
    )
  );

-- ── Blind "which is brighter" votes ─────────────────────────────────────────
create table public.resonance_votes (
  id               uuid primary key default gen_random_uuid(),
  pair_id          uuid not null references public.dataset_pairs(id) on delete cascade,
  voter_id         uuid not null references auth.users(id) on delete cascade,
  chosen_sample_id uuid not null references public.dataset_samples(id) on delete cascade,
  created_at       timestamptz not null default now(),
  unique (pair_id, voter_id)
);
create index resonance_votes_pair_idx on public.resonance_votes (pair_id);

alter table public.resonance_votes enable row level security;

create policy "read own resonance votes or as pair owner"
  on public.resonance_votes for select
  using (
    voter_id = auth.uid()
    or exists (select 1 from public.dataset_pairs p where p.id = pair_id and p.speaker_id = auth.uid())
  );

-- Vote as yourself, only on a public pair, choosing a sample that belongs to it.
create policy "cast resonance vote"
  on public.resonance_votes for insert
  with check (
    voter_id = auth.uid()
    and exists (select 1 from public.dataset_pairs p where p.id = pair_id and p.visibility = 'public')
    and exists (select 1 from public.dataset_samples s where s.id = chosen_sample_id and s.pair_id = resonance_votes.pair_id)
  );

grant select, insert on public.resonance_votes to authenticated;
grant select, insert, update, delete on public.resonance_votes to service_role;

-- ── Comments on pairs ───────────────────────────────────────────────────────
create table public.pair_comments (
  id         uuid primary key default gen_random_uuid(),
  pair_id    uuid not null references public.dataset_pairs(id) on delete cascade,
  author_id  uuid not null references auth.users(id) on delete cascade,
  body       text not null check (char_length(body) between 1 and 2000),
  created_at timestamptz not null default now()
);
create index pair_comments_pair_idx on public.pair_comments (pair_id, created_at);

alter table public.pair_comments enable row level security;

create policy "read pair comments on visible pairs"
  on public.pair_comments for select
  using (
    author_id = auth.uid()
    or exists (select 1 from public.dataset_pairs p
               where p.id = pair_id and (p.visibility = 'public' or p.speaker_id = auth.uid()))
  );
create policy "comment on public pairs"
  on public.pair_comments for insert
  with check (
    author_id = auth.uid()
    and exists (select 1 from public.dataset_pairs p where p.id = pair_id and p.visibility = 'public')
  );
create policy "delete own pair comment or moderate own pair"
  on public.pair_comments for delete
  using (
    author_id = auth.uid()
    or exists (select 1 from public.dataset_pairs p where p.id = pair_id and p.speaker_id = auth.uid())
  );

grant select, insert, delete on public.pair_comments to authenticated;
grant select, insert, update, delete on public.pair_comments to service_role;

-- ── Aggregate vote stats per pair (anonymous; safe to expose) ───────────────
create function public.resonance_pair_stats(p_pair_id uuid)
returns table (label text, votes bigint)
language sql
stable
security definer
set search_path = ''
as $$
  select ds.label, count(v.id)
  from public.dataset_samples ds
  left join public.resonance_votes v on v.chosen_sample_id = ds.id
  where ds.pair_id = p_pair_id
  group by ds.label
  order by ds.label;
$$;

grant execute on function public.resonance_pair_stats(uuid) to authenticated, service_role;
