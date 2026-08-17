<script lang="ts">
  import { nuru } from '$lib/store.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import Mixer from './components/Mixer.svelte';
  import SoundGrid from './components/SoundGrid.svelte';
  import TimerPanel from './components/TimerPanel.svelte';
  import PresetPanel from './components/PresetPanel.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import NookMode from './components/NookMode.svelte';
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
{/if}

{#if nuru.nookMode}
  <NookMode />
{/if}

<Toasts />

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
