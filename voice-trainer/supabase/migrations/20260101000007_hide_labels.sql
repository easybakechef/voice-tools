-- ============================================================================
--  Hide resonance labels until a viewer has voted.
--
--  The deep/bright label leaked two ways: in the audio filename and via the
--  readable dataset_samples.label column. This moves the label into a separate
--  sample_labels table whose RLS only reveals it to the speaker, or to someone
--  who has already cast a vote on the pair. (Filenames are made opaque in app
--  code; this migration handles the column.)
-- ============================================================================

create table public.sample_labels (
  sample_id  uuid primary key references public.dataset_samples(id) on delete cascade,
  pair_id    uuid not null references public.dataset_pairs(id) on delete cascade,
  speaker_id uuid not null references auth.users(id) on delete cascade,
  label      text not null check (label in ('deep', 'bright')),
  unique (pair_id, label)
);
create index sample_labels_pair_idx on public.sample_labels (pair_id);

-- Carry over any existing labels, then drop the exposed column (CASCADE also
-- removes the old unique(pair_id, label) constraint that depended on it).
insert into public.sample_labels (sample_id, pair_id, speaker_id, label)
  select id, pair_id, speaker_id, label from public.dataset_samples;
alter table public.dataset_samples drop column label cascade;

alter table public.sample_labels enable row level security;

-- The gate: speaker always; everyone else only after they've voted on the pair.
create policy "read labels if owner or voted"
  on public.sample_labels for select
  using (
    speaker_id = auth.uid()
    or exists (
      select 1 from public.resonance_votes v
      where v.pair_id = sample_labels.pair_id and v.voter_id = auth.uid()
    )
  );
create policy "insert own labels"
  on public.sample_labels for insert
  with check (speaker_id = auth.uid());

grant select, insert on public.sample_labels to authenticated;
grant select, insert, update, delete on public.sample_labels to service_role;

-- Stats now read the label from sample_labels (SECURITY DEFINER bypasses RLS,
-- so aggregates work without exposing per-sample labels to the caller).
create or replace function public.resonance_pair_stats(p_pair_id uuid)
returns table (label text, votes bigint)
language sql
stable
security definer
set search_path = ''
as $$
  select sl.label, count(v.id)
  from public.sample_labels sl
  left join public.resonance_votes v on v.chosen_sample_id = sl.sample_id
  where sl.pair_id = p_pair_id
  group by sl.label
  order by sl.label;
$$;
