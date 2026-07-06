<script lang="ts">
  import { onMount } from 'svelte';
  import { auth, type OAuthProvider } from '$lib/supabase/auth.svelte.js';

  let mode     = $state<'create' | 'signin'>('create');
  let email    = $state('');
  let name     = $state('');
  let password = $state('');
  let busy     = $state(false);
  let msg      = $state('');
  let error    = $state('');

  onMount(() => auth.init());

  // Seed the name field from the loaded profile.
  $effect(() => { name = auth.displayName ?? ''; });

  async function emailAction() {
    const e = email.trim();
    if (!e || busy) return;
    busy = true; error = ''; msg = '';
    try {
      if (mode === 'create') {
        await auth.claimWithEmail(e);
        msg = `Almost there — check ${e} and click the link to finish creating your account. Your recordings will carry over.`;
      } else {
        await auth.signInWithEmail(e);
        msg = `Magic link sent to ${e}. Open it to sign in.`;
      }
    } catch (err) { error = String(err); } finally { busy = false; }
  }

  async function oauth(provider: OAuthProvider) {
    busy = true; error = ''; msg = '';
    try {
      if (mode === 'create') await auth.claimWithProvider(provider);
      else await auth.signInWithProvider(provider);
      // browser redirects to the provider on success
    } catch (err) { error = String(err); busy = false; }
  }

  async function saveName() {
    busy = true; error = ''; msg = '';
    try { await auth.updateDisplayName(name); msg = 'Display name saved.'; }
    catch (err) { error = String(err); } finally { busy = false; }
  }

  async function savePassword() {
    if (!password) return;
    busy = true; error = ''; msg = '';
    try { await auth.setPassword(password); password = ''; msg = 'Password set — you can now log in with email + password.'; }
    catch (err) { error = String(err); } finally { busy = false; }
  }

  async function doSignOut() {
    busy = true; error = ''; msg = '';
    try { await auth.signOut(); mode = 'create'; }
    catch (err) { error = String(err); } finally { busy = false; }
  }
</script>

<div class="card">
  {#if auth.isSignedIn}
    <div class="card-label">Your Account</div>
    <p class="who">Signed in as <strong>{auth.email ?? auth.label}</strong></p>

    <div class="field">
      <span class="lbl">Display name</span>
      <div class="row">
        <input bind:value={name} placeholder="Your name" maxlength="60" />
        <button class="primary" onclick={saveName} disabled={busy}>Save</button>
      </div>
    </div>

    <div class="field">
      <span class="lbl">Set / change password <em>(optional)</em></span>
      <div class="row">
        <input type="password" bind:value={password} placeholder="New password" autocomplete="new-password" />
        <button class="ghost" onclick={savePassword} disabled={busy || !password}>Set</button>
      </div>
    </div>

    <button class="signout" onclick={doSignOut} disabled={busy}>Sign out</button>
  {:else}
    <div class="card-label">{mode === 'create' ? 'Create an account' : 'Sign in'}</div>
    <p class="intro">
      {#if mode === 'create'}
        Save your recordings and choose a username. You'll stay signed in on this device and can
        return any time via email or Google/Discord.
      {:else}
        Sign in to an existing account.
      {/if}
    </p>

    <div class="field">
      <span class="lbl">Email</span>
      <div class="row">
        <input type="email" bind:value={email} placeholder="you@example.com"
          onkeydown={(e) => { if (e.key === 'Enter') emailAction(); }} />
        <button class="primary" onclick={emailAction} disabled={busy || !email.trim()}>
          {mode === 'create' ? 'Continue' : 'Send link'}
        </button>
      </div>
    </div>

    <div class="or"><span>or</span></div>

    <div class="providers">
      <button class="oauth google" onclick={() => oauth('google')} disabled={busy}>Continue with Google</button>
      <button class="oauth discord" onclick={() => oauth('discord')} disabled={busy}>Continue with Discord</button>
    </div>

    <button class="switch" onclick={() => { mode = mode === 'create' ? 'signin' : 'create'; error = ''; msg = ''; }}>
      {mode === 'create' ? 'Already have an account? Sign in' : 'New here? Create an account'}
    </button>
  {/if}

  {#if msg}<p class="msg">{msg}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .who { font-size: 0.9rem; margin-bottom: 0.5rem; }
  .intro { font-size: 0.82rem; color: var(--muted); line-height: 1.55; margin-bottom: 0.5rem; }

  .field { margin-top: 0.85rem; }
  .lbl { display: block; font-size: 0.74rem; color: var(--muted); margin-bottom: 0.35rem; }
  .lbl em { font-style: normal; opacity: 0.7; }
  .row { display: flex; gap: 0.5rem; }
  input {
    flex: 1; background: #12122a; border: 1px solid var(--border); border-radius: 6px;
    padding: 0.5rem 0.75rem; color: var(--text); font-size: 0.875rem; min-width: 0;
  }
  input:focus { outline: none; border-color: var(--trans-pink); }

  .primary {
    border: none; border-radius: 6px; cursor: pointer; white-space: nowrap;
    background: var(--trans-pink); color: #0d0d24; font-weight: 700; font-size: 0.85rem; padding: 0.5rem 1rem;
  }
  .primary:disabled { opacity: 0.5; cursor: default; }
  .ghost {
    border: 1px solid var(--border); border-radius: 6px; background: transparent;
    color: var(--muted); font-weight: 600; font-size: 0.85rem; padding: 0.5rem 1rem; cursor: pointer; white-space: nowrap;
  }
  .ghost:disabled { opacity: 0.5; cursor: default; }

  .or { display: flex; align-items: center; text-align: center; color: var(--muted); font-size: 0.72rem; margin: 1rem 0 0.75rem; }
  .or::before, .or::after { content: ''; flex: 1; border-top: 1px solid var(--border); }
  .or span { padding: 0 0.6rem; }

  .providers { display: flex; flex-direction: column; gap: 0.5rem; }
  .oauth {
    border-radius: 8px; padding: 0.6rem 1rem; font-size: 0.85rem; font-weight: 700; cursor: pointer;
    border: 1px solid var(--border); background: #12122a; color: var(--text);
  }
  .oauth:hover:not(:disabled) { filter: brightness(1.2); }
  .oauth.google  { border-color: #4285f4; }
  .oauth.discord { border-color: #5865f2; }
  .oauth:disabled { opacity: 0.5; cursor: default; }

  .switch { margin-top: 1rem; background: none; border: none; color: var(--trans-blue); font-size: 0.8rem; cursor: pointer; padding: 0; }
  .switch:hover { text-decoration: underline; }

  .signout { margin-top: 1.25rem; border: 1px solid rgba(231,76,111,0.4); background: rgba(231,76,111,0.08); color: #e74c6f; border-radius: 8px; padding: 0.5rem 1.1rem; font-weight: 600; font-size: 0.85rem; cursor: pointer; }
  .signout:disabled { opacity: 0.5; cursor: default; }

  .msg { margin-top: 0.85rem; color: var(--trans-blue); font-size: 0.82rem; line-height: 1.5; }
  .error { margin-top: 0.85rem; color: #e74c3c; font-size: 0.82rem; }
</style>
