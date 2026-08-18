<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  let {
    title,
    onclose,
    width = 420,
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

<button class="scrim" transition:fade={{ duration: 200 }} onclick={onclose} aria-label="Close {title}"
></button>

<div
  class="drawer"
  style:--w="{width}px"
  role="dialog"
  aria-modal="true"
  aria-label={title}
  transition:fly={{ x: 460, duration: 340, easing: cubicOut, opacity: 1 }}
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
    background: rgba(6, 7, 8, 0.5);
    cursor: default;
  }

  .drawer {
    position: fixed;
    top: var(--titlebar-h);
    right: 0;
    bottom: 0;
    z-index: 51;
    width: var(--w);
    max-width: 100vw;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--line);
    background: linear-gradient(180deg, var(--s-700), var(--s-800) 42%);
    box-shadow: -24px 0 60px -20px rgba(0, 0, 0, 0.6);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-5) var(--sp-4) var(--sp-3) var(--sp-5);
    border-bottom: 1px solid var(--line-soft);
    flex: 0 0 auto;
  }

  h2 {
    font: var(--t-title);
    letter-spacing: -0.01em;
  }

  .close {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--ink-40);
  }
  .close:hover,
  .close:focus-visible {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.08);
  }

  .body {
    flex: 1;
    padding: var(--sp-5);
    overflow-y: auto;
  }
</style>
