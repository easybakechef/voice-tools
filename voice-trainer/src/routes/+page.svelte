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

  type Tab = 'live' | 'playback' | 'library' | 'community' | 'rank' | 'dataset';
  let activeTab = $state<Tab>('live');

  function switchTab(tab: Tab) {
    if (tab === activeTab) return;
    engine.stopAll();
    activeTab = tab;
  }
</script>

<svelte:head>
  <title>Voice Resonance Trainer</title>
</svelte:head>

<header>
  <h1>Voice Resonance Trainer</h1>
  <p>Real-time pitch &amp; resonance feedback for voice feminization</p>
</header>

<div class="tabs">
  <button class="tab-btn" class:active={activeTab === 'live'}     onclick={() => switchTab('live')}>
    🎤 Live Recording
  </button>
  <button class="tab-btn" class:active={activeTab === 'playback'} onclick={() => switchTab('playback')}>
    ▶ Playback Mode
  </button>
  <button class="tab-btn" class:active={activeTab === 'library'}  onclick={() => switchTab('library')}>
    📁 Library
  </button>
  <button class="tab-btn" class:active={activeTab === 'community'} onclick={() => switchTab('community')}>
    💬 Community
  </button>
  <button class="tab-btn" class:active={activeTab === 'rank'} onclick={() => switchTab('rank')}>
    🏆 Rank
  </button>
  <button class="tab-btn" class:active={activeTab === 'dataset'} onclick={() => switchTab('dataset')}>
    📚 Dataset
  </button>
</div>

{#if activeTab === 'live'}
  <RecordPanel />
  <AnalysisView />
  <SnapshotCard />
{:else if activeTab === 'playback'}
  <PlaybackPanel />
  <AnalysisView />
  <SnapshotCard clipMode={true} />
{:else if activeTab === 'library'}
  <LibraryPanel />
{:else if activeTab === 'community'}
  <CommunityPanel />
{:else if activeTab === 'rank'}
  <RankPanel />
{:else}
  <DatasetPanel />
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
