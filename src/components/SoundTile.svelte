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

  const layers = $derived.by(() => {
    const art = cover ? `url("${cover}")` : 'none';
    if (active) {
      const tint = `linear-gradient(${sound.accent}e0, ${sound.accent}e0)`;
      return { image: `${tint}, ${art}`, blend: 'screen, normal' };
    }
    const scrim = 'linear-gradient(to top, rgba(4, 5, 7, 0.7) 0%, rgba(4, 5, 7, 0) 52%)';
    const dim = 'linear-gradient(rgba(7, 8, 10, 0.66), rgba(7, 8, 10, 0.66))';
    return { image: `${scrim}, ${dim}, ${art}`, blend: 'normal, normal, normal' };
  });
</script>

<button
  class="tile"
  class:active
  class:loading
  style:--accent={sound.accent}
  style:background-image={layers.image}
  style:background-blend-mode={layers.blend}
  aria-pressed={active}
  aria-label="{sound.name}{active ? ', playing' : ''}"
  onclick={ontoggle}
>
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
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    text-align: left;
    overflow: hidden;
    background-size: cover;
    background-position: center;
    background-repeat: no-repeat;
    box-shadow: var(--e-2);
    transition:
      transform var(--dur-3) var(--ease-spring),
      box-shadow var(--dur-3) var(--ease-out),
      filter var(--dur-3) var(--ease-out);
  }

  .tile:hover,
  .tile:focus-visible {
    transform: translateY(-4px);
    box-shadow: var(--e-3);
    z-index: 2;
    filter: brightness(1.28);
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
    filter: brightness(1.02) contrast(1.12) saturate(1.28);
  }


  .tile.active:hover,
  .tile.active:focus-visible {
    transform: translateY(-4px);
    filter: brightness(1.02) contrast(1.12) saturate(1.28);
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

  .tile.active .label {
    color: rgba(10, 11, 13, 0.9);
    text-shadow: 0 1px 10px rgba(255, 255, 255, 0.4);
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
  }
</style>
