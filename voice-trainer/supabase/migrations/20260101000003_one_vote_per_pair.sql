-- ============================================================================
--  Strict pairwise voting: one vote per (voter, unordered pair) + undo support
-- ============================================================================

-- A voter may judge a given pair at most once, regardless of which side was
-- shown as A vs B. least()/greatest() normalize the pair to an unordered key.
create unique index comparison_votes_one_per_pair
  on public.comparison_votes (
    set_id,
    attribute,
    voter_id,
    least(recording_a, recording_b),
    greatest(recording_a, recording_b)
  );

-- Let voters undo (delete) their own vote — powers the "Undo last" button.
create policy "delete own vote"
  on public.comparison_votes for delete
  using (voter_id = auth.uid());

grant delete on public.comparison_votes to authenticated;
