<script lang="ts">
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { nuru } from '$lib/store.svelte';
  import Icon from './Icon.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each nuru.toasts as t (t.id)}
    <div
      class="toast"
      class:error={t.tone === 'error'}
      in:fly={{ y: 16, duration: 280, easing: cubicOut }}
      out:fly={{ y: 10, duration: 160, easing: cubicOut }}
    >
      {#if t.tone === 'error'}
        <Icon name="warn" size={15} />
      {:else}
        <Icon name="check" size={15} stroke={2.2} />
      {/if}
      <span>{t.text}</span>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    bottom: 22px;
    transform: translateX(-50%);
    z-index: 80;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-2);
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    max-width: 460px;
    padding: 10px 16px;
    border-radius: var(--r-pill);
    font: var(--t-body);
    color: var(--ink);
    background: rgba(28, 30, 36, 0.94);
    backdrop-filter: blur(12px);
    box-shadow: var(--e-3);
    outline: 1px solid var(--line);
    outline-offset: -1px;
  }

  .toast.error {
    color: #ffd9d6;
    background: rgba(58, 26, 26, 0.94);
    outline-color: rgba(255, 95, 87, 0.3);
  }
</style>
