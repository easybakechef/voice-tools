import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  // better-sqlite3 is a native module; keep it external to Vite's bundling
  ssr: { external: ['better-sqlite3'] }
});
