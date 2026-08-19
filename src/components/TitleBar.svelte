<script lang="ts">
  import { nuru } from '$lib/store.svelte';
  import { win, IS_PREVIEW } from '$lib/bridge';
  import { VERSION } from '$lib/version';
  import NuruMark from './NuruMark.svelte';
  import Icon from './Icon.svelte';

  const playing = $derived(nuru.playing && nuru.layers.length > 0);

  const timerLabel = $derived.by(() => {
    if (nuru.timer.kind !== 'running') return null;
    const s = Math.ceil(nuru.timerRemainingMs / 1000);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    return h > 0
      ? `${h}:${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`
      : `${m}:${String(sec).padStart(2, '0')}`;
  });

  function panel(name: 'presets' | 'timer' | 'settings') {
    nuru.activePanel = nuru.activePanel === name ? 'none' : name;
  }
</script>

<header class="titlebar u-drag">
  <div class="brand">
    <NuruMark size={19} pulse={playing} />
    <span class="wordmark">Nuru</span>
    <span class="version u-numeric">{VERSION}</span>
    {#if IS_PREVIEW}
      <span class="tag" title="Running in a browser - the audio engine is not attached">
        preview
      </span>
    {/if}
  </div>

  <div class="tools u-nodrag">
    <button
      class="u-pressable nook"
      onclick={() => (nuru.nookMode = true)}
      title="Cozy mode (F11)"
    >
      <span class="cozy-text">Cozy Mode</span>
    </button>
    {#if nuru.update?.available}
      <button
        class="u-pressable pill update"
        onclick={() => (nuru.activePanel = 'settings')}
        title="Nuru {nuru.update.available.version} is available"
      >
        <span class="pill-text">Update</span>
      </button>
    {/if}
    {#if timerLabel}
      <button
        class="u-pressable pill live"
        onclick={() => panel('timer')}
        title="Sleep timer running"
      >
        <Icon name="timer" size={14} />
        <span class="u-numeric">{timerLabel}</span>
      </button>
    {:else}
      <button
        class="u-pressable tool"
        class:on={nuru.activePanel === 'timer'}
        onclick={() => panel('timer')}
        aria-label="Timers"
        title="Timers and schedule"
      >
        <Icon name="timer" size={16} />
      </button>
    {/if}


    <button
      class="u-pressable tool"
      class:on={nuru.activePanel === 'settings'}
      onclick={() => panel('settings')}
      aria-label="Settings"
      title="Settings"
    >
      <Icon name="settings" size={16} />
    </button>
  </div>

  <div class="window-buttons u-nodrag">
    <button class="wb" onclick={() => win.minimize()} aria-label="Minimize">
      <Icon name="minimize" size={15} stroke={1.4} />
    </button>
    <button class="wb" onclick={() => win.toggleMaximize()} aria-label="Maximize">
      <Icon name="maximize" size={13} stroke={1.4} />
    </button>
    <button class="wb close" onclick={() => win.close()} aria-label="Close">
      <Icon name="close" size={14} stroke={1.4} />
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: var(--titlebar-h);
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding-left: var(--sp-4);
    border-bottom: 1px solid var(--line-soft);
    background: var(--s-800);
    position: relative;
    z-index: 40;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }

  .wordmark {
    font: var(--t-subtitle);
    font-weight: 600;
    letter-spacing: 0.01em;

    position: relative;
    top: 2px;
  }

  .version {
    font: var(--t-caption);
    color: var(--ink-25);
    letter-spacing: 0.03em;
  }

  .tag {
    font: var(--t-caption);
    letter-spacing: var(--tracking-caps);
    text-transform: uppercase;
    color: var(--nuru);
    background: var(--nuru-ghost);
    padding: 2px 7px;
    border-radius: var(--r-pill);
  }

  .tools {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .tool {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--ink-40);
  }
  .tool:hover,
  .tool:focus-visible {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.07);
  }
  .tool.on {
    color: var(--nuru);
    background: var(--nuru-ghost);
  }

  .nook {
    height: 26px;
    padding: 0 11px;
    margin-inline: 4px;
    border-radius: var(--r-pill);
    font: var(--t-label);
    font-family: var(--font-display);
    color: var(--ink-60);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .cozy-text {
    position: relative;
    top: 2px;
  }

  .nook:hover,
  .nook:focus-visible {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.07);
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border-radius: var(--r-pill);
    font: var(--t-caption);
  }
  .pill-text {
    position: relative;
    top: 2px;
  }

  .pill.update {
    color: var(--ink-inverse);
    background: var(--nuru);
    font-family: var(--font-display);
    font-weight: 600;
    box-shadow: 0 0 18px -4px var(--nuru);
  }

  .pill.live {
    color: var(--ink-inverse);
    background: var(--nuru);
    box-shadow: 0 0 18px -4px var(--nuru);
  }


  .window-buttons {
    display: flex;
    margin-left: var(--sp-3);
  }

  .wb {
    width: 46px;
    height: var(--titlebar-h);
    display: grid;
    place-items: center;
    color: var(--ink-60);
    transition:
      background-color var(--dur-1) linear,
      color var(--dur-1) linear;
  }
  .wb:hover,
  .wb:focus-visible {
    background: rgba(255, 255, 255, 0.08);
    color: var(--ink);
  }
  .wb:active {
    background: rgba(255, 255, 255, 0.04);
  }
  .wb.close:hover {
    background: #e81123;
    color: #fff;
  }
  .wb.close:active {
    background: #f1707a;
    color: #fff;
  }
</style>
