# Authentication

## Model

Everyone starts as an **anonymous** user (see `src/lib/supabase/client.ts`), so the
app is usable with zero friction. From the **Account** tab they can:

- **Create an account / save their work** — this *claims* the anonymous session
  (keeping all recordings/pairs/votes) via:
  - **Magic-link email** — enter email, click the link.
  - **Google** or **Discord** — one click.
- **Sign in** to an existing account (magic link or Google/Discord).
- Set a **display name** and an optional **password**, or **sign out** (which
  drops back to a fresh anonymous session).

Auth state + actions live in `src/lib/supabase/auth.svelte.ts`; the UI is
`src/lib/components/AccountPanel.svelte`.

## Local testing (email — works out of the box)

Email auth runs **entirely locally** — no real mail is sent:

- `enable_confirmations = true`, so claims/sign-ins send a verification email.
- Those emails are caught by **Mailpit → http://localhost:54324**. Open it and
  click the link to complete sign-in / confirm a claim.
- `npx supabase db reset` wipes users (they live in the auth schema); seed test
  users in `supabase/seed-community.mjs` if you want stable logins.

So you can fully exercise magic-link sign-in and the anonymous→permanent claim
with just Mailpit.

## Enabling Google & Discord

These need OAuth apps **you** register (I can't create them). The code/UI is
already wired — you just provide credentials and flip a flag.

### 1. Register the apps

For **both**, set the **Authorized redirect URI** to your Supabase auth callback:

- Local: `http://127.0.0.1:54321/auth/v1/callback`
- Production: `https://<your-project-ref>.supabase.co/auth/v1/callback`

- **Google** — [Google Cloud Console](https://console.cloud.google.com/) →
  APIs & Services → Credentials → *Create OAuth client ID* (type: Web). Copy the
  **Client ID** and **Client secret**.
- **Discord** — [Discord Developer Portal](https://discord.com/developers/applications)
  → *New Application* → OAuth2 → add the redirect URL. Copy the **Client ID** and
  **Client Secret**.

### 2. Provide the credentials locally

Export these before `npx supabase start` (the CLI reads `env(...)` from your
shell / project `.env`):

```bash
export GOOGLE_CLIENT_ID="…"
export GOOGLE_SECRET="…"
export DISCORD_CLIENT_ID="…"
export DISCORD_SECRET="…"
```

### 3. Turn them on

In `supabase/config.toml`, set `enabled = true` under `[auth.external.google]`
and `[auth.external.discord]`, then restart:

```bash
npx supabase stop && npx supabase start
```

The **Continue with Google / Discord** buttons in the Account tab will now work.
(Until then they return a "provider not enabled" error.)

> Note: `skip_nonce_check = true` is already set for Google — required for local
> sign-in.

## Production

Configure the providers in the **Supabase dashboard** (Authentication →
Providers) instead of config.toml, set the **Site URL** + redirect URLs to your
deployed origin, and keep `enable_confirmations` on. The data model and UI are
identical — no code changes needed.
