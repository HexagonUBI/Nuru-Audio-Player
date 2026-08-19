<script lang="ts">
  import { nuru } from '$lib/store.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import Mixer from './components/Mixer.svelte';
  import SoundGrid from './components/SoundGrid.svelte';
  import TimerPanel from './components/TimerPanel.svelte';
  import PresetPanel from './components/PresetPanel.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import CreditsPanel from './components/CreditsPanel.svelte';
  import NookMode from './components/NookMode.svelte';
  import LoadingScreen from './components/LoadingScreen.svelte';
  import ChangelogPanel from './components/ChangelogPanel.svelte';
  import Toasts from './components/Toasts.svelte';

  $effect(() => {
    void nuru.init();
  });

  function keydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    const typing =
      target?.tagName === 'INPUT' ||
      target?.tagName === 'TEXTAREA' ||
      target?.isContentEditable;

    if (typing) return;

    if (e.code === 'Space') {
      e.preventDefault();
      void nuru.togglePlaying();
    } else if (e.key === 'F11') {
      e.preventDefault();
      nuru.nookMode = !nuru.nookMode;
    } else if (e.key === 'Escape') {
      if (nuru.activePanel !== 'none') nuru.activePanel = 'none';
    } else if (e.key === '/') {
      e.preventDefault();
      document.querySelector<HTMLInputElement>('.search input')?.focus();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="app">
  <TitleBar />
  <main>
    <Mixer />
    <SoundGrid />
  </main>
</div>

{#if nuru.activePanel === 'timer'}
  <TimerPanel />
{:else if nuru.activePanel === 'presets'}
  <PresetPanel />
{:else if nuru.activePanel === 'settings'}
  <SettingsPanel />
{:else if nuru.activePanel === 'credits'}
  <CreditsPanel />
{/if}

{#if nuru.nookMode}
  <NookMode />
{/if}

<ChangelogPanel />

<Toasts />
<LoadingScreen />

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--app-bg);
  }

  main {
    flex: 1;
    min-height: 0;
    display: flex;
  }
</style>
