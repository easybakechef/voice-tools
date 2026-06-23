# Supabase backend

The recording storage was migrated from the local Rust/SQLite server to Supabase
(Postgres + Auth + Storage). The Rust backend in `crates/server` is now **legacy**
and no longer used by the app (kept for reference; run via `npm run server:legacy`).

## Architecture

- **Frontend** — unchanged SvelteKit SPA; all DSP still runs client-side in WASM.
- **Data-access layer** — `src/lib/data/recordings.ts` is the *only* module that
  talks to Supabase for recordings. Components import from it, never from the
  Supabase client directly. To migrate off Supabase later, reimplement this one
  file plus `src/lib/supabase/client.ts`.
- **Auth** — `src/lib/supabase/client.ts` signs every browser in **anonymously**
  so the app keeps its no-login UX while RLS scopes data per user. Real
  email/OAuth sign-in can upgrade the same session later.
- **Database** — `supabase/migrations/20260101000000_init.sql` defines:
  - `profiles` (1:1 with `auth.users`): `tier`, `storage_limit_bytes`,
    `bytes_used`, `approved` (waitlist gate, default-open for now).
  - `recordings`: owned by `user_id`, with a `visibility` flag (`private` /
    `public`) for future sharing.
  - **RLS** on both tables: you can only read/modify your own rows (plus read
    recordings others mark `public`).
  - A private **Storage bucket** (`recordings`) for audio, owner-only access,
    files namespaced `<user_id>/<uuid>.webm`, served via short-lived signed URLs.
  - A per-user **byte quota** enforced by a trigger (`check_recording_quota`).
  - `comments` (migration `…01_feedback.sql`): feedback on recordings. RLS lets
    anyone comment on a recording marked `public`, lets the owner read all
    feedback on their own recording, and adds a Storage policy so a public
    recording's audio is playable by others. Surfaced in the UI via the
    Library → **Feedback** tab (receive feedback, toggle public/private) and the
    **Community** tab (leave feedback on others' shared recordings).

## Prerequisites

- Docker Desktop running (the local Supabase stack runs in containers).
- Node 18+.

## First-time setup

```bash
npm install
cp .env.example .env        # already present; values are the standard local ones

# Start the local Supabase stack (Postgres, Auth, Storage, Studio…)
npm run db:start            # prints API URL + anon key — should match .env

# Apply migrations to a fresh local DB
npm run db:reset
```

If the printed `anon key` / `API URL` differ from `.env`, copy them into
`.env` (`PUBLIC_SUPABASE_URL`, `PUBLIC_SUPABASE_ANON_KEY`).

## Day-to-day

```bash
npm run db:start   # backend (once per session)
npm run dev        # frontend
```

- Supabase Studio (DB browser / policy viewer): http://localhost:54323
- Stop the stack: `npm run db:stop`

## Testing data isolation

```bash
npm run db:test    # runs supabase/tests/rls_isolation_test.sql (pgTAP)
```

This proves user B cannot read user A's private recording — the privacy
guarantee you can't verify by hand while logged in as yourself.

## Deploying to a hosted project (later)

1. Create a project at supabase.com.
2. `npx supabase link --project-ref <ref>`
3. `npx supabase db push` to apply migrations.
4. Set `PUBLIC_SUPABASE_URL` / `PUBLIC_SUPABASE_ANON_KEY` to the hosted values in
   your deploy environment.

## Turning on the waitlist (cost cap)

`profiles.approved` defaults to `true`. To require manual approval before users
can upload, edit the `insert own recordings` policy in the migration to add:

```sql
and (select approved from public.profiles where id = auth.uid())
```

…and flip the `approved` default to `false`. Approve users with the
`service_role` key (server-side only).
