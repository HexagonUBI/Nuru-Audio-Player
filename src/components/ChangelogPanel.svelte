<script lang="ts">
  import { nuru } from '$lib/store.svelte';
  import Drawer from './Drawer.svelte';
  import Markdown from './Markdown.svelte';
  import NuruMark from './NuruMark.svelte';

  const release = $derived(nuru.changelog);
</script>

{#if release}
  <Drawer title="What's new" onclose={() => nuru.dismissChangelog()} width={460}>
    <div class="head">
      <NuruMark size={34} />
      <div class="who">
        <span class="ver">Nuru {release.version}</span>
        {#if release.title && release.title !== release.version}
          <span class="sub">{release.title}</span>
        {/if}
      </div>
    </div>

    {#if release.notes.trim()}
      <Markdown source={release.notes} />
    {:else}
      <p class="empty">This release came without notes.</p>
    {/if}

    <button class="u-pressable solid" onclick={() => nuru.dismissChangelog()}>
      Got it
    </button>
  </Drawer>
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding-bottom: var(--sp-4);
    margin-bottom: var(--sp-4);
    border-bottom: 1px solid var(--line-soft);
  }

  .who {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .ver {
    font: var(--t-title);
    font-family: var(--font-display);
  }

  .sub {
    font: var(--t-caption);
    color: var(--ink-40);
  }

  .empty {
    font: var(--t-body);
    color: var(--ink-40);
  }

  .solid {
    margin-top: var(--sp-6);
    width: 100%;
    padding: 11px;
    border-radius: var(--r-sm);
    background: var(--ink);
    color: var(--ink-inverse);
    font-weight: 600;
  }
  .solid:hover {
    background: #fff;
  }
</style>
