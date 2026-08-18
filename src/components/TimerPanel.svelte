<script lang="ts">
  import { nuru } from '$lib/store.svelte';
  import Panel from './Panel.svelte';

  const PRESETS = [10, 20, 30, 45, 60, 90, 120];

  let custom = $state(45);

  const running = $derived(nuru.timer.kind === 'running');

  const remaining = $derived.by(() => {
    const s = Math.ceil(nuru.timerRemainingMs / 1000);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    return h > 0
      ? `${h}:${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`
      : `${m}:${String(sec).padStart(2, '0')}`;
  });

  const progress = $derived(
    nuru.timer.kind === 'running' ? 1 - nuru.timerRemainingMs / nuru.timer.totalMs : 0,
  );
</script>

<Panel title="Sleep timer" onclose={() => (nuru.activePanel = 'none')} width={340}>
  {#if running}
    <div class="running">
      <div class="dial">
        <div class="ring" style:--p={progress}></div>
        <span class="u-numeric time">{remaining}</span>
      </div>
      <p class="hint">Everything fades out and pauses when this reaches zero.</p>
      <button class="u-pressable solid danger" onclick={() => nuru.stopTimer()}>
        Cancel timer
      </button>
    </div>
  {:else}
    <p class="hint">Pause everything after...</p>
    <div class="grid">
      {#each PRESETS as m (m)}
        <button
          class="u-pressable choice"
          onclick={() => {
            nuru.startTimer(m);
            nuru.activePanel = 'none';
          }}
        >
          <span class="u-numeric n">{m}</span>
          <span class="u">min</span>
        </button>
      {/each}
    </div>

    <div class="custom">
      <label for="timer-custom">Custom</label>
      <input
        id="timer-custom"
        type="number"
        min="1"
        max="600"
        bind:value={custom}
        class="u-numeric"
      />
      <span class="u">min</span>
      <button
        class="u-pressable solid"
        onclick={() => {
          nuru.startTimer(Math.min(600, Math.max(1, custom)));
          nuru.activePanel = 'none';
        }}
      >
        Start
      </button>
    </div>
  {/if}
</Panel>

<style>
  .hint {
    font: var(--t-body);
    color: var(--ink-40);
    margin-bottom: var(--sp-4);
    line-height: 1.6;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--sp-2);
    margin-bottom: var(--sp-5);
  }

  .choice {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    padding: 10px 0;
    border-radius: var(--r-md);
    background: rgba(255, 255, 255, 0.04);
    box-shadow: inset 0 0 0 1px var(--line-soft);
  }
  .choice:hover {
    background: rgba(255, 255, 255, 0.09);
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }

  .n {
    font: var(--t-subtitle);
    font-family: var(--font-display);
    color: var(--ink);
  }

  .u {
    font: var(--t-caption);
    color: var(--ink-40);
  }

  .custom {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding-top: var(--sp-4);
    border-top: 1px solid var(--line-soft);
  }
  .custom label {
    font: var(--t-label);
    color: var(--ink-60);
    margin-right: auto;
  }
  .custom input {
    width: 64px;
    padding: 7px 9px;
    border-radius: var(--r-sm);
    background: rgba(255, 255, 255, 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    text-align: right;
  }
  .custom input:focus {
    box-shadow: inset 0 0 0 1px var(--nuru);
  }

  .solid {
    padding: 8px 14px;
    border-radius: var(--r-sm);
    background: var(--ink);
    color: var(--ink-inverse);
    font-weight: 600;
  }
  .solid:hover {
    background: #fff;
  }
  .solid.danger {
    background: var(--danger-dim);
    color: var(--danger);
    width: 100%;
    padding: 10px;
  }
  .solid.danger:hover {
    background: color-mix(in srgb, var(--danger) 26%, transparent);
  }

  .running {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-4);
  }

  .dial {
    position: relative;
    display: grid;
    place-items: center;
  }

  .ring {
    width: 148px;
    height: 148px;
    border-radius: 50%;
    background: conic-gradient(
      var(--nuru) calc(var(--p) * 360deg),
      rgba(255, 255, 255, 0.07) 0
    );
    mask: radial-gradient(circle, transparent 60px, #000 61px);
    -webkit-mask: radial-gradient(circle, transparent 60px, #000 61px);
  }

  .time {
    position: absolute;
    font: var(--t-display);
    font-family: var(--font-display);
    color: var(--ink);
  }
</style>
