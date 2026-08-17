<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  import { nuru } from '$lib/store.svelte';
  import SoundTile from './SoundTile.svelte';
  import Icon from './Icon.svelte';

  const loadingIds = $derived(new Set(nuru.layers.filter((l) => l.loading).map((l) => l.soundId)));
</script>

<section class="grid-pane">
  <div class="toolbar">
    <label class="search">
      <Icon name="search" size={15} />
      <input
        type="text"
        placeholder="Search sounds"
        bind:value={nuru.search}
        spellcheck="false"
        autocomplete="off"
      />
      {#if nuru.search}
        <button
          class="u-pressable clear-search"
          onclick={() => (nuru.search = '')}
          aria-label="Clear search"
        >
          <Icon name="close" size={12} stroke={2} />
        </button>
      {/if}
    </label>

    <div class="chips" role="group" aria-label="Filter by tag">
      <button
        class="chip u-pressable"
        class:on={nuru.filter === ''}
        onclick={() => (nuru.filter = '')}
      >
        All
      </button>
      {#each nuru.tags as tag (tag)}
        <button
          class="chip u-pressable"
          class:on={nuru.filter === tag}
          onclick={() => (nuru.filter = nuru.filter === tag ? '' : tag)}
        >
          {tag}
        </button>
      {/each}
    </div>
  </div>

  <div class="scroll">
    {#if nuru.loading}
      <div class="grid" aria-hidden="true">
        {#each { length: 8 } as _, i (i)}
          <div class="skeleton" style:animation-delay="{i * 60}ms"></div>
        {/each}
      </div>
    {:else if nuru.visibleSounds.length === 0}
      <p class="none" in:fade={{ duration: 200 }}>
        {nuru.sounds.length === 0
          ? 'No sound packs found. Nuru looks in its resources folder and in your app data folder.'
          : 'Nothing matches that.'}
      </p>
    {:else}
      <div class="grid">
        {#each nuru.visibleSounds as sound, i (sound.id)}
          <div
            in:fly={{ y: 14, duration: 300, delay: Math.min(i * 22, 260), easing: cubicOut }}
            out:fade={{ duration: 120 }}
          >
            <SoundTile
              {sound}
              active={nuru.activeIds.has(sound.id)}
              loading={loadingIds.has(sound.id)}
              ontoggle={() => nuru.toggle(sound.id)}
            />
          </div>
        {/each}
      </div>
    {/if}
  </div>
</section>

<style>
  .grid-pane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-4) var(--sp-6);
    border-bottom: 1px solid var(--line-soft);
    flex: 0 0 auto;
  }

  .search {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: 0 10px;
    height: 32px;
    width: 230px;
    flex: 0 0 auto;
    border-radius: var(--r-sm);
    background: rgba(255, 255, 255, 0.05);
    color: var(--ink-40);
    transition:
      background-color var(--dur-2) var(--ease-out),
      box-shadow var(--dur-2) var(--ease-out),
      color var(--dur-2) var(--ease-out);
  }
  .search:focus-within {
    background: rgba(255, 255, 255, 0.08);
    /* Inset, so it reads as the field's own edge rather than a ring around it. */
    box-shadow: inset 0 0 0 1px var(--line-strong);
    color: var(--ink-60);
  }

  .search input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    color: var(--ink);
  }
  .search input::placeholder {
    color: var(--ink-25);
  }

  .clear-search {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--ink-40);
    background: rgba(255, 255, 255, 0.08);
  }
  .clear-search:hover {
    color: var(--ink);
  }

  .chips {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    scrollbar-width: none;
    padding-bottom: 1px;
  }
  .chips::-webkit-scrollbar {
    display: none;
  }

  .chip {
    padding: 5px 12px;
    border-radius: var(--r-pill);
    font: var(--t-caption);
    letter-spacing: 0.02em;
    text-transform: capitalize;
    color: var(--ink-40);
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line);
    white-space: nowrap;
  }
  .chip:hover,
  .chip:focus-visible {
    color: var(--ink-80);
    background: rgba(255, 255, 255, 0.05);
  }
  .chip.on {
    color: var(--ink-inverse);
    background: var(--ink);
    box-shadow: none;
  }

  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: var(--sp-5) var(--sp-6) var(--sp-8);
  }

  .grid {
    display: grid;
    gap: var(--tile-gap);
    grid-template-columns: repeat(auto-fill, minmax(var(--tile-min), 1fr));
  }

  .skeleton {
    aspect-ratio: 1 / 1;
    border-radius: var(--tile-radius);
    background: linear-gradient(
      100deg,
      var(--s-700) 20%,
      var(--s-600) 40%,
      var(--s-700) 60%
    );
    background-size: 240% 100%;
    animation: shimmer 1.4s var(--ease-in-out) infinite;
  }

  @keyframes shimmer {
    from {
      background-position: 160% 0;
    }
    to {
      background-position: -60% 0;
    }
  }

  .none {
    font: var(--t-body);
    color: var(--ink-40);
    max-width: 46ch;
    line-height: 1.7;
    padding: var(--sp-6) 0;
  }
</style>
