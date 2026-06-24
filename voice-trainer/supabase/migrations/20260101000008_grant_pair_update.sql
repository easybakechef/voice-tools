-- The resonance-community migration added an "update own pairs" RLS policy so
-- owners could publish/unpublish, but never granted the UPDATE privilege itself,
-- so setPairVisibility() hit "permission denied for table dataset_pairs".
grant update on public.dataset_pairs to authenticated;
