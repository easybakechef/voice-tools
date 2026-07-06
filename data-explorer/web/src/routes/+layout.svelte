<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  let { children } = $props();

  const links = [
    { href: '/', label: 'Overview' },
    { href: '/explorer', label: 'Sample explorer' },
    { href: '/hard-cases', label: 'Where it struggles' }
  ];
  const active = (href: string) =>
    href === '/' ? $page.url.pathname === '/' : $page.url.pathname.startsWith(href);
</script>

<div class="layout">
  <nav class="side">
    <h1>Resonance Explorer</h1>
    <div class="sub">voice data · Rust + SvelteKit</div>
    {#each links as l}
      <a href={l.href} class:active={active(l.href)}>{l.label}</a>
    {/each}
  </nav>
  <main>
    {@render children()}
  </main>
</div>
