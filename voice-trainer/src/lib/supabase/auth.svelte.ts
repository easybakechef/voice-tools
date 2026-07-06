// ============================================================================
//  Auth state + actions
//
//  Reactive wrapper around Supabase Auth. Everyone starts anonymous (see
//  client.ts); from here they can CLAIM that anonymous session into a permanent
//  account (keeping their data) via magic-link email or Google/Discord, or sign
//  into an existing account. Also covers display-name editing and password set.
// ============================================================================

import { supabase } from './client.js';
import type { User } from '@supabase/supabase-js';

export type OAuthProvider = 'google' | 'discord';

function redirectTo(): string | undefined {
  return typeof window !== 'undefined' ? window.location.origin : undefined;
}

class AuthStore {
  user = $state<User | null>(null);
  displayName = $state<string | null>(null);
  #initialized = false;

  get isAnonymous(): boolean { return this.user?.is_anonymous ?? false; }
  get isSignedIn(): boolean { return !!this.user && !this.user.is_anonymous; }
  get email(): string | null { return this.user?.email ?? null; }

  /** Friendly label for the current identity. */
  get label(): string {
    if (this.isAnonymous || !this.user) return 'Guest';
    return this.displayName || this.email || 'Account';
  }

  init(): void {
    if (this.#initialized || typeof window === 'undefined') return;
    this.#initialized = true;
    void supabase.auth.getUser().then(({ data }) => this.#setUser(data.user));
    supabase.auth.onAuthStateChange((_event, session) => { void this.#setUser(session?.user ?? null); });
  }

  async #setUser(u: User | null): Promise<void> {
    this.user = u;
    if (u && !u.is_anonymous) await this.#loadProfile();
    else this.displayName = null;
  }

  async #loadProfile(): Promise<void> {
    const { data } = await supabase
      .from('profiles')
      .select('display_name')
      .eq('id', this.user!.id)
      .maybeSingle();
    this.displayName = data?.display_name ?? null;
  }

  // ── Claim the anonymous session into a permanent account (keeps data) ───────
  /** Add an email to the current (anonymous) account; user confirms via email. */
  async claimWithEmail(email: string): Promise<void> {
    const { error } = await supabase.auth.updateUser({ email });
    if (error) throw new Error(error.message);
  }

  /** Link a Google/Discord identity to the current account (redirects out). */
  async claimWithProvider(provider: OAuthProvider): Promise<void> {
    const { error } = await supabase.auth.linkIdentity({ provider, options: { redirectTo: redirectTo() } });
    if (error) throw new Error(error.message);
  }

  // ── Sign in to an existing account (replaces the anonymous session) ─────────
  async signInWithEmail(email: string): Promise<void> {
    const { error } = await supabase.auth.signInWithOtp({ email, options: { emailRedirectTo: redirectTo() } });
    if (error) throw new Error(error.message);
  }

  async signInWithProvider(provider: OAuthProvider): Promise<void> {
    const { error } = await supabase.auth.signInWithOAuth({ provider, options: { redirectTo: redirectTo() } });
    if (error) throw new Error(error.message);
  }

  // ── Account management ──────────────────────────────────────────────────────
  async updateDisplayName(name: string): Promise<void> {
    const trimmed = name.trim();
    const { error } = await supabase.from('profiles').update({ display_name: trimmed || null }).eq('id', this.user!.id);
    if (error) throw new Error(error.message);
    this.displayName = trimmed || null;
  }

  async setPassword(password: string): Promise<void> {
    const { error } = await supabase.auth.updateUser({ password });
    if (error) throw new Error(error.message);
  }

  /** Sign out, then drop back to a fresh anonymous session so the app stays usable. */
  async signOut(): Promise<void> {
    await supabase.auth.signOut();
    await supabase.auth.signInAnonymously();
  }
}

export const auth = new AuthStore();
