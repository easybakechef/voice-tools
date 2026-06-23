-- ============================================================================
--  Paired resonance dataset
--
--  Speakers read a one-sentence phrase twice — once with DEEP/low resonance and
--  once with BRIGHT resonance — and submit both takes together. The two takes
--  share one uuid (the dataset_pairs id), and each audio file is named
--  "<uuid>-deep-resonance.webm" / "<uuid>-bright-resonance.webm".
--
--  Audio reuses the private `recordings` bucket under
--  "<speaker_id>/dataset/<uuid>-<label>-resonance.webm", which the existing
--  owner-only storage policies already cover (no new storage policy needed).
-- ============================================================================

-- ── sample_phrases : the sentences to read (curated, read-only to users) ─────
create table public.sample_phrases (
  id         uuid primary key default gen_random_uuid(),
  text       text not null,
  sort       int  not null default 0,
  active     boolean not null default true,
  created_at timestamptz not null default now()
);

alter table public.sample_phrases enable row level security;
create policy "read active phrases" on public.sample_phrases for select using (true);
grant select on public.sample_phrases to authenticated;
grant select, insert, update, delete on public.sample_phrases to service_role;

insert into public.sample_phrases (text, sort) values
  ('The quick brown fox jumps over the lazy dog.', 1),
  ('She sells seashells by the seashore on sunny mornings.', 2),
  ('I really think we should leave before it starts raining.', 3),
  ('Could you please pass me the salt and pepper?', 4),
  ('The sunset painted the sky in shades of orange and pink.', 5),
  ('My favorite season is autumn because of the cool, crisp air.', 6);

-- ── dataset_pairs : one recorded pair; id IS the shared sample uuid ──────────
create table public.dataset_pairs (
  id         uuid primary key default gen_random_uuid(),
  speaker_id uuid not null references auth.users(id) on delete cascade,
  phrase_id  uuid not null references public.sample_phrases(id),
  created_at timestamptz not null default now()
);
create index dataset_pairs_speaker_idx on public.dataset_pairs (speaker_id, created_at desc);

alter table public.dataset_pairs enable row level security;
create policy "read own pairs"   on public.dataset_pairs for select using (speaker_id = auth.uid());
create policy "insert own pairs"  on public.dataset_pairs for insert with check (speaker_id = auth.uid());
create policy "delete own pairs"  on public.dataset_pairs for delete using (speaker_id = auth.uid());
grant select, insert, delete on public.dataset_pairs to authenticated;
grant select, insert, update, delete on public.dataset_pairs to service_role;

-- ── dataset_samples : the two takes (deep + bright) of a pair ───────────────
create table public.dataset_samples (
  id           uuid primary key default gen_random_uuid(),
  pair_id      uuid not null references public.dataset_pairs(id) on delete cascade,
  speaker_id   uuid not null references auth.users(id) on delete cascade,
  label        text not null check (label in ('deep', 'bright')),
  storage_path text not null,
  created_at   timestamptz not null default now(),
  unique (pair_id, label)
);
create index dataset_samples_pair_idx on public.dataset_samples (pair_id);

alter table public.dataset_samples enable row level security;
create policy "read own samples"  on public.dataset_samples for select using (speaker_id = auth.uid());
create policy "insert own samples" on public.dataset_samples for insert with check (speaker_id = auth.uid());
create policy "delete own samples" on public.dataset_samples for delete using (speaker_id = auth.uid());
grant select, insert, delete on public.dataset_samples to authenticated;
grant select, insert, update, delete on public.dataset_samples to service_role;
