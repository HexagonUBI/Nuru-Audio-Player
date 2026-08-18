<script lang="ts">
  import { fade } from 'svelte/transition';
  import { nuru } from '$lib/store.svelte';
  import NuruMark from './NuruMark.svelte';

  const boot = $derived(nuru.boot);

  const shown = $derived(Math.round(boot.progress * 100));
</script>

{#if boot.visible}
  <div
    class="loading"
    style:--accent={boot.accent}
    out:fade={{ duration: 420 }}
    role="status"
    aria-live="polite"
  >
    <div class="middle">
      <div class="mark"><NuruMark size={64} pulse /></div>
      <h1>{boot.mode === 'update' ? 'Updating Nuru' : 'Nuru'}</h1>
      <p class="line">{boot.line}</p>

      <div class="bar" aria-hidden="true">
        <div class="fill" style:width="{shown}%"></div>
      </div>

      <p class="detail">
        {#if boot.error}
          <span class="err">{boot.error}</span>
        {:else}
          {boot.label || ' '}
        {/if}
      </p>
    </div>

    <div class="glow" aria-hidden="true"></div>
  </div>
{/if}

<style>
  .loading {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    background: var(--s-900);
    overflow: hidden;
  }

  .glow {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 720px;
    height: 720px;
    transform: translate(-50%, -50%);
    background: radial-gradient(
      circle,
      color-mix(in srgb, var(--accent) 16%, transparent) 0%,
      transparent 62%
    );
    pointer-events: none;
  }

  .middle {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: min(420px, 78vw);
    text-align: center;
  }

  .mark {
    --nuru: var(--accent);
    --nuru-bright: color-mix(in srgb, var(--accent) 62%, #fff);
    margin-bottom: var(--sp-4);
  }

  h1 {
    font: 600 30px/1.1 var(--font-display);
    letter-spacing: 0.01em;
    margin-bottom: var(--sp-2);
  }

  .line {
    font: var(--t-body);
    color: var(--ink-40);
    margin-bottom: var(--sp-6);
    min-height: 1.5em;
  }

  .bar {
    width: 100%;
    height: 5px;
    border-radius: var(--r-pill);
    background: rgba(255, 255, 255, 0.07);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: var(--r-pill);
    background: var(--accent);
    box-shadow: 0 0 16px -2px var(--accent);
    transition: width var(--dur-4) var(--ease-out);
  }

  .detail {
    margin-top: var(--sp-3);
    font: var(--t-caption);
    color: var(--ink-25);
    min-height: 1.4em;
  }

  .err {
    color: var(--danger);
  }
</style>
