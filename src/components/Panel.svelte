<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  let {
    title,
    onclose,
    width = 380,
    children,
  }: { title: string; onclose: () => void; width?: number; children: Snippet } = $props();

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<!-- The scrim closes the panel and dims what is behind it. It is a button rather
     than a div with a click handler so it is reachable without a mouse. -->
<button class="scrim" transition:fade={{ duration: 180 }} onclick={onclose} aria-label="Close {title}"
></button>

<div
  class="panel"
  style:--w="{width}px"
  role="dialog"
  aria-modal="true"
  aria-label={title}
  transition:fly={{ y: -14, duration: 260, easing: cubicOut }}
>
  <header>
    <h2>{title}</h2>
    <button class="u-pressable close" onclick={onclose} aria-label="Close">
      <Icon name="close" size={15} stroke={2} />
    </button>
  </header>
  <div class="body">
    {@render children()}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: rgba(6, 7, 8, 0.55);
    backdrop-filter: blur(2px);
    cursor: default;
  }

  .panel {
    position: fixed;
    z-index: 51;
    top: calc(var(--titlebar-h) + 10px);
    right: 14px;
    width: var(--w);
    max-width: calc(100vw - 28px);
    max-height: calc(100vh - var(--titlebar-h) - 28px);
    display: flex;
    flex-direction: column;
    border-radius: var(--r-lg);
    background: var(--s-700);
    box-shadow: var(--e-4);
    /* A one-pixel top highlight — the standard trick for making a dark panel
       look lit from above rather than pasted on. */
    outline: 1px solid var(--line);
    outline-offset: -1px;
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-4) var(--sp-4) var(--sp-3) var(--sp-5);
    border-bottom: 1px solid var(--line-soft);
    flex: 0 0 auto;
  }

  h2 {
    font: var(--t-title);
  }

  .close {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--ink-40);
  }
  .close:hover {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.08);
  }

  .body {
    padding: var(--sp-4) var(--sp-5) var(--sp-5);
    overflow-y: auto;
  }
</style>
