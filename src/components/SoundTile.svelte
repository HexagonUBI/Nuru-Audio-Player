<script lang="ts">
  import type { SoundEntry } from '$lib/types';
  import { coverUrl } from '$lib/bridge';
  import Icon from './Icon.svelte';

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
  style:background-image={cover ? `url("${cover}")` : 'none'}
  aria-pressed={active}
  aria-label="{sound.name}{active ? ', playing' : ''}"
  onclick={ontoggle}
>
  <!-- Sits over the cover and under the label. Off: a dark scrim so white text
       stays legible on any artwork. On: the sound's accent in screen blend,
       which lights the photograph rather than painting over it. -->
  <span class="wash" aria-hidden="true"></span>

  <!-- Separate from .wash so the bloom can scale independently of the colour
       fade — the colour arrives fast, the glow spreads after it. -->
  <span class="bloom" aria-hidden="true"></span>

  <span class="label">{sound.name}</span>

  {#if loading}
    <span class="badge" aria-hidden="true"><span class="spinner"></span></span>
  {:else if active}
    <span class="badge on" aria-hidden="true"><Icon name="check" size={13} stroke={2.4} /></span>
  {/if}
</button>

<style>
  .tile {
    position: relative;
    aspect-ratio: 1 / 1;
    width: 100%;
    border-radius: var(--tile-radius);
    overflow: hidden;
    background-size: cover;
    background-position: center;
    text-align: left;
    isolation: isolate;
    box-shadow: var(--e-2);
    transform: translateZ(0);
    transition:
      transform var(--dur-3) var(--ease-spring),
      box-shadow var(--dur-3) var(--ease-out);
  }

  /* Keyboard focus lifts the tile exactly as hover does — no ring. */
  .tile:hover,
  .tile:focus-visible {
    transform: translateY(-4px) scale(1.018);
    box-shadow: var(--e-3);
    z-index: 2;
  }

  .tile:active {
    transform: translateY(-1px) scale(0.985);
    transition-duration: var(--dur-1);
  }

  /* Active tiles sit above their neighbours so the glow spills over them rather
     than being clipped by them. This is the detail that makes the grid read as
     lit rather than merely tinted. */
  .tile.active {
    z-index: 3;
    box-shadow:
      var(--e-2),
      var(--glow-lg);
  }
  .tile.active:hover {
    transform: translateY(-4px) scale(1.018);
    box-shadow:
      var(--e-3),
      var(--glow-lg);
  }

  .wash {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      165deg,
      rgba(10, 11, 13, 0.5) 0%,
      rgba(10, 11, 13, 0.78) 62%,
      rgba(10, 11, 13, 0.9) 100%
    );
    transition: opacity var(--dur-3) var(--ease-out);
  }

  .tile:hover .wash,
  .tile:focus-visible .wash {
    opacity: 0.86;
  }

  .bloom {
    position: absolute;
    inset: 0;
    background: var(--accent);
    mix-blend-mode: screen;
    opacity: 0;
    transform: scale(1.6);
    transition:
      opacity var(--dur-3) var(--ease-out),
      transform var(--dur-4) var(--ease-out);
  }

  .tile.active .bloom {
    opacity: 0.9;
    transform: scale(1);
  }

  .tile.active .wash {
    opacity: 0.34;
  }

  .label {
    position: relative;
    display: block;
    margin: 22px 20px;
    /* Smaller than the old wide tile carried, so two-word names still fit on a
       square without wrapping to three lines. */
    font: 600 22px/1.2 var(--font-display);
    letter-spacing: -0.01em;
    color: var(--ink);
    text-shadow: 0 2px 18px rgba(0, 0, 0, 0.6);
    transition:
      color var(--dur-3) var(--ease-out),
      text-shadow var(--dur-3) var(--ease-out);
  }

  /* On the lit accent the label flips to ink so it keeps its contrast. */
  .tile.active .label {
    color: rgba(12, 13, 15, 0.86);
    text-shadow: 0 1px 12px rgba(255, 255, 255, 0.28);
  }

  .badge {
    position: absolute;
    right: 16px;
    bottom: 16px;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: rgba(12, 13, 15, 0.62);
    color: var(--ink);
    backdrop-filter: blur(8px);
    animation: badge-in var(--dur-4) var(--ease-spring);
  }

  .badge.on {
    background: rgba(12, 13, 15, 0.78);
  }

  @keyframes badge-in {
    from {
      opacity: 0;
      transform: scale(0.4);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .spinner {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.25);
    border-top-color: var(--ink);
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tile:hover,
    .tile.active:hover {
      transform: none;
    }
  }
</style>
