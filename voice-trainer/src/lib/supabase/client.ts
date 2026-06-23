import { createClient, type SupabaseClient } from '@supabase/supabase-js';
import { env } from '$env/dynamic/public';

// Config comes from env (see .env / .env.example). The fallbacks are the
// standard Supabase *local* dev values printed by `npx supabase start` — fine to
// ship since the local anon key is public and only works against localhost.
const SUPABASE_URL =
  env.PUBLIC_SUPABASE_URL ?? 'http://127.0.0.1:54321';
const SUPABASE_ANON_KEY =
  env.PUBLIC_SUPABASE_ANON_KEY ??
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0';

export const supabase: SupabaseClient = createClient(SUPABASE_URL, SUPABASE_ANON_KEY, {
  auth: {
    persistSession: true,
    autoRefreshToken: true,
    detectSessionInUrl: true,
  },
});

// ── Identity bootstrap ──────────────────────────────────────────────────────
// The app has no login screen yet, but RLS needs an authenticated user. We give
// every browser an *anonymous* identity so data is scoped per-user from day one.
// Later, real email/OAuth sign-in can upgrade the same session in place.
let resolveAuthReady!: () => void;
export const authReady: Promise<void> = new Promise((r) => (resolveAuthReady = r));

async function ensureSession(): Promise<void> {
  try {
    const { data } = await supabase.auth.getSession();
    if (!data.session) {
      const { error } = await supabase.auth.signInAnonymously();
      if (error) console.error('[supabase] anonymous sign-in failed:', error.message);
    }
  } catch (e) {
    console.error('[supabase] session bootstrap failed:', e);
  } finally {
    resolveAuthReady();
  }
}

// Browser-only: never attempt sign-in during SSR/prerender.
if (typeof window !== 'undefined') {
  void ensureSession();
} else {
  resolveAuthReady();
}

/** Resolve the current user's id, waiting for the anonymous session if needed. */
export async function currentUserId(): Promise<string> {
  await authReady;
  const { data } = await supabase.auth.getUser();
  if (!data.user) throw new Error('No authenticated user (anonymous sign-in may be disabled)');
  return data.user.id;
}
