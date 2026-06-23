<script lang="ts">
  import { engine } from '$lib/audio/engine.svelte.js';
  import RecordPanel   from '$lib/components/RecordPanel.svelte';
  import PlaybackPanel from '$lib/components/PlaybackPanel.svelte';
  import LibraryPanel   from '$lib/components/LibraryPanel.svelte';
  import CommunityPanel from '$lib/components/CommunityPanel.svelte';
  import RankPanel      from '$lib/components/RankPanel.svelte';
  import DatasetPanel   from '$lib/components/DatasetPanel.svelte';
  import AnalysisView   from '$lib/components/AnalysisView.svelte';
  import SnapshotCard   from '$lib/components/SnapshotCard.svelte';

  type Section = 'recording' | 'community' | 'rank' | 'dataset';
  type RecTab  = 'live' | 'library' | 'playback';

  const NAV = [
    { id: 'recording', label: 'Recording', icon: '🎙' },
    { id: 'community', label: 'Community', icon: '💬' },
    { id: 'rank',      label: 'Rank',      icon: '🏆' },
    { id: 'dataset',   label: 'Dataset',   icon: '📚' },
  ] as const;

  let section = $state<Section>('recording');
  let recTab  = $state<RecTab>('live');

  function selectSection(s: Section) {
    if (s === section) return;
    engine.stopAll();
    section = s;
    if (s === 'recording') recTab = 'live'; // always open on Live Recording
  }

  function selectRecTab(t: RecTab) {
    if (t === recTab) return;
    engine.stopAll();
    recTab = t;
  }
</script>

<svelte:head>
  <title>Voice Resonance Trainer</title>
</svelte:head>

<header>
  <h1>Voice Resonance Trainer</h1>
  <p>Real-time pitch &amp; resonance feedback for voice feminization</p>
</header>

<div class="layout">
  <aside class="sidebar">
    {#each NAV as item (item.id)}
      <button class="nav-item" class:active={section === item.id} onclick={() => selectSection(item.id)}>
        <span class="nav-icon">{item.icon}</span>{item.label}
      </button>
    {/each}
  </aside>

  <main class="content">
    {#if section === 'recording'}
      <div class="tabs">
        <button class="tab-btn" class:active={recTab === 'live'}     onclick={() => selectRecTab('live')}>
          🎤 Live Recording
        </button>
        <button class="tab-btn" class:active={recTab === 'library'}  onclick={() => selectRecTab('library')}>
          📁 Library
        </button>
        <button class="tab-btn" class:active={recTab === 'playback'} onclick={() => selectRecTab('playback')}>
          ▶ Playback Mode
        </button>
      </div>

      {#if recTab === 'live'}
        <RecordPanel />
        <AnalysisView />
        <SnapshotCard />
      {:else if recTab === 'library'}
        <LibraryPanel />
      {:else}
        <PlaybackPanel />
        <AnalysisView />
        <SnapshotCard clipMode={true} />
      {/if}

      <p class="info">
        Feminine speech typically has a fundamental pitch above
        <strong style="color:var(--trans-pink)">165 Hz</strong>
        and a raised second formant (F2) — the "brightness" associated with forward tongue placement.
        The shaded region on the pitch bar marks the target range (165–255 Hz).<br /><br />
        <em>
          Playback demos stream public-domain LibriVox recordings from the Internet Archive.
          Upload your own recording (e.g. from a voice memo app) for personal voice analysis.
        </em>
      </p>
    {:else if section === 'community'}
      <CommunityPanel />
    {:else if section === 'rank'}
      <RankPanel />
    {:else}
      <DatasetPanel />
    {/if}
  </main>
</div>

<style>
  .layout {
    display: flex;
    align-items: flex-start;
    gap: 1.5rem;
    width: 100%;
    max-width: 980px;
  }

  .sidebar {
    position: sticky;
    top: 1rem;
    flex-shrink: 0;
    width: 200px;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 0.6rem;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    text-align: left;
    padding: 0.65rem 0.8rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    font-size: 0.92rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .nav-item:hover { color: var(--text); background: rgba(255,255,255,0.04); }
  .nav-item.active {
    background: linear-gradient(135deg, rgba(91,206,250,0.14), rgba(245,169,184,0.14));
    color: var(--text);
  }
  .nav-icon { font-size: 1.05rem; line-height: 1; }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
  }

  @media (max-width: 680px) {
    .layout { flex-direction: column; max-width: 760px; }
    .sidebar { position: static; width: 100%; flex-direction: row; flex-wrap: wrap; }
    .nav-item { flex: 1 1 auto; justify-content: center; }
  }
</style>
