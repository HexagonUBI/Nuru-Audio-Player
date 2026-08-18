<script lang="ts">
  import type { SoundEntry } from '$lib/types';
  import { coverUrl } from '$lib/bridge';

  let {
    sound,
    active,
    loading = false,
    ontoggle,
  }: {
    sound: SoundEntry;
    active: boolean;
    loading?: boolean;
    ontoggle: () => void;
  } = $props();

  const cover = $derived(coverUrl(sound));
</script>

<button
  class="tile"
  class:active
  class:loading
  style:--accent={sound.accent}
  aria-pressed={active}
  aria-label="{sound.name}{active ? ', playing' : ''}"
  onclick={ontoggle}
>
  <span
    class="art"
    style:background-image={cover ? `url("${cover}")` : 'none'}
    aria-hidden="true"
  ></span>
  <span class="dim" aria-hidden="true"></span>
  <span class="tint" aria-hidden="true"></span>
  <span class="scrim" aria-hidden="true"></span>

  <span class="label">{sound.name}</span>

  {#if loading}
    <span class="spinner" aria-hidden="true"></span>
  {/if}
</button>

<style>
  .tile {
    position: relative;
    width: var(--tile-size);
    height: var(--tile-size);
    border-radius: var(--tile-radius);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    text-align: left;
    box-shadow: var(--e-2);
    isolation: isolate;
    transition:
      transform var(--dur-3) var(--ease-spring),
      box-shadow var(--dur-3) var(--ease-out);
  }

  .art,
  .dim,
  .tint,
  .scrim {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    pointer-events: none;
  }

  .art {
    background-size: cover;
    background-position: center;
    transition:
      transform var(--dur-4) var(--ease-out),
      filter var(--dur-3) var(--ease-out);
  }

  .dim {
    background: #07080a;
    opacity: 0.66;
    transition: opacity var(--dur-3) var(--ease-out);
  }

  .tint {
    background: var(--accent);
    mix-blend-mode: screen;
    opacity: 0;
    transition: opacity var(--dur-3) var(--ease-out);
  }

  .scrim {
    background: linear-gradient(to top, rgba(4, 5, 7, 0.7) 0%, rgba(4, 5, 7, 0) 52%);
  }

  .tile:hover,
  .tile:focus-visible {
    transform: translateY(-4px);
    box-shadow: var(--e-3);
    z-index: 2;
  }

  .tile:hover .dim,
  .tile:focus-visible .dim {
    opacity: 0.5;
  }

  .tile:hover .art,
  .tile:focus-visible .art {
    transform: scale(1.05);
  }

  .tile:active {
    transform: translateY(-1px) scale(0.985);
    transition-duration: var(--dur-1);
  }

  .tile.active {
    z-index: 3;
    box-shadow:
      var(--e-2),
      0 4px 34px -6px var(--accent);
  }

  .tile.active .dim {
    opacity: 0;
  }

  .tile.active .art {
    filter: brightness(1.16) contrast(1.2) saturate(1.45);
  }

  .tile.active .tint {
    opacity: 0.92;
  }

  .tile.active .scrim {
    background: linear-gradient(to top, rgba(255, 255, 255, 0.34) 0%, rgba(255, 255, 255, 0) 48%);
  }

  .tile.active .label {
    color: rgba(10, 11, 13, 0.88);
    text-shadow: 0 1px 10px rgba(255, 255, 255, 0.45);
  }

  .tile.active:hover {
    transform: translateY(-4px);
  }

  .label {
    position: relative;
    margin: 0 16px 14px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: var(--tile-label);
    line-height: 1.2;
    letter-spacing: -0.01em;
    color: var(--ink);
    text-shadow:
      0 1px 3px rgba(0, 0, 0, 0.85),
      0 2px 16px rgba(0, 0, 0, 0.55);
  }

  .spinner {
    position: absolute;
    right: 12px;
    top: 12px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.25);
    border-top-color: #fff;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tile:hover,
    .tile:focus-visible,
    .tile.active:hover {
      transform: none;
    }
    .tile:hover .art,
    .tile:focus-visible .art {
      transform: none;
    }
  }
</style>
