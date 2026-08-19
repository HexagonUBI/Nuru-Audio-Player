<script lang="ts">
  import { nuru, slotLabel } from '$lib/store.svelte';
  import Icon from './Icon.svelte';

  let brush = $state<string | null>(null);
  let painting = $state(false);
  let painted = $state<'set' | 'clear'>('set');

  const hours = Array.from({ length: 24 }, (_, h) => h);

  $effect(() => {
    if (brush && !nuru.presets.some((p) => p.id === brush)) brush = null;
    if (!brush && nuru.presets.length) brush = nuru.presets[0].id;
  });

  function accentOf(presetId: string | null): string {
    if (!presetId) return 'transparent';
    const preset = nuru.presetById.get(presetId);
    const first = preset?.layers[0];
    return (first && nuru.byId.get(first.soundId)?.accent) || 'var(--nuru)';
  }

  function nameOf(presetId: string | null): string {
    if (!presetId) return 'Nothing';
    return nuru.presetById.get(presetId)?.name ?? 'Missing mix';
  }

  function apply(slot: number) {
    const current = nuru.schedule[slot] ?? null;
    if (painted === 'clear') {
      nuru.setSlot(slot, null);
      return;
    }
    nuru.setSlot(slot, current === brush ? null : brush);
  }

  function begin(slot: number) {
    if (!brush) return;
    painting = true;
    painted = (nuru.schedule[slot] ?? null) === brush ? 'clear' : 'set';
    apply(slot);
  }

  function extend(slot: number) {
    if (!painting) return;
    apply(slot);
  }

  const total = $derived(nuru.schedule.filter((s) => s !== null).length * 30);
</script>

<svelte:window onpointerup={() => (painting = false)} />

<section class="head">
  <div class="toggle-row">
    <div>
      <strong>Run mixes on a clock</strong>
      <p class="note">
        Each block is 30 minutes. Blocks that touch and share a mix play as one stretch.
      </p>
    </div>
    <button
      class="u-pressable sw"
      class:on={nuru.scheduleEnabled}
      role="switch"
      aria-checked={nuru.scheduleEnabled}
      aria-label="Enable the schedule"
      onclick={() => nuru.setScheduleEnabled(!nuru.scheduleEnabled)}
    >
      <span class="knob"></span>
    </button>
  </div>

  {#if nuru.scheduleEnabled}
    <p class="now">
      {#if nuru.scheduleBlock.id}
        <span class="dot" style:background={accentOf(nuru.scheduleBlock.id)}></span>
        <strong>{nameOf(nuru.scheduleBlock.id)}</strong>
        until {nuru.scheduleBlock.endsAt}
      {:else}
        <span class="dot empty"></span>
        Nothing scheduled right now
      {/if}
    </p>
    {#if nuru.scheduleNext}
      <p class="note next">
        Next at {nuru.scheduleNext.at}:
        {nuru.scheduleNext.stops ? 'stops' : (nuru.scheduleNext.preset?.name ?? 'a deleted mix')}
      </p>
    {/if}
  {/if}
</section>

{#if !nuru.presets.length}
  <p class="none">
    Save a mix first. The schedule plays saved mixes, so there is nothing to place yet.
  </p>
{:else}
  <div class="palette">
    {#each nuru.presets as p (p.id)}
      <button
        class="u-pressable swatch"
        class:on={brush === p.id}
        style:--c={accentOf(p.id)}
        onclick={() => (brush = p.id)}
        title={`Paint with ${p.name}`}
      >
        <span class="chip"></span>
        <span class="pname">{p.name}</span>
      </button>
    {/each}
  </div>

  <p class="note tip">
    Click a block to place <strong>{nameOf(brush)}</strong>, drag across several to fill
    them, click a placed block again to clear it.
  </p>

  <div class="clock">
    {#each hours as h (h)}
      <div class="row">
        {#each [h * 2, h * 2 + 1] as slot (slot)}
          <button
            class="u-pressable cell"
            class:filled={nuru.schedule[slot]}
            class:live={nuru.scheduleSlot === slot && nuru.scheduleEnabled}
            style:--c={accentOf(nuru.schedule[slot] ?? null)}
            onpointerdown={() => begin(slot)}
            onpointerenter={() => extend(slot)}
            title={nameOf(nuru.schedule[slot] ?? null)}
            aria-label={`${slotLabel(slot)}, ${nameOf(nuru.schedule[slot] ?? null)}`}
          >
            <span class="at u-numeric">{slotLabel(slot)}</span>
          </button>
        {/each}
      </div>
    {/each}
  </div>

  <div class="foot">
    <span class="note">{total ? `${(total / 60).toFixed(1)} hours filled` : 'Nothing placed'}</span>
    <button class="u-pressable ghost" disabled={!total} onclick={() => nuru.clearSchedule()}>
      <Icon name="warn" size={12} />
      Clear all
    </button>
  </div>
{/if}

<style>
  .head {
    margin-bottom: var(--sp-4);
  }

  .toggle-row {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-3);
  }
  .toggle-row strong {
    font: var(--t-label);
    color: var(--ink);
  }

  .note {
    font: var(--t-caption);
    color: var(--ink-40);
    line-height: 1.55;
  }

  .sw {
    flex: none;
    width: 38px;
    height: 22px;
    margin-top: 2px;
    border-radius: var(--r-pill);
    background: rgba(255, 255, 255, 0.09);
    box-shadow: inset 0 0 0 1px var(--line);
    position: relative;
  }
  .sw .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--ink-40);
    transition: transform var(--m-fast, 140ms) ease, background var(--m-fast, 140ms) ease;
  }
  .sw.on {
    background: color-mix(in srgb, var(--nuru) 40%, transparent);
  }
  .sw.on .knob {
    transform: translateX(16px);
    background: var(--ink);
  }

  .now {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin-top: var(--sp-3);
    font: var(--t-body);
    color: var(--ink-60);
  }
  .now strong {
    color: var(--ink);
  }
  .next {
    margin-top: 2px;
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex: none;
  }
  .dot.empty {
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }

  .none {
    font: var(--t-body);
    color: var(--ink-40);
    line-height: 1.6;
  }

  .palette {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
    padding-top: var(--sp-3);
    border-top: 1px solid var(--line-soft);
  }

  .swatch {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px 5px 7px;
    border-radius: var(--r-pill);
    background: rgba(255, 255, 255, 0.04);
    box-shadow: inset 0 0 0 1px var(--line-soft);
  }
  .swatch:hover {
    background: rgba(255, 255, 255, 0.09);
  }
  .swatch.on {
    background: rgba(255, 255, 255, 0.12);
    box-shadow: inset 0 0 0 1px var(--c);
  }
  .swatch .chip {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    background: var(--c);
  }
  .pname {
    font: var(--t-caption);
    color: var(--ink-60);
    max-width: 108px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .swatch.on .pname {
    color: var(--ink);
  }

  .tip {
    margin: var(--sp-3) 0;
  }
  .tip strong {
    color: var(--ink-60);
  }

  .clock {
    display: flex;
    flex-direction: column;
    gap: 3px;
    touch-action: none;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px;
    align-items: stretch;
  }

  .cell {
    position: relative;
    height: 24px;
    border-radius: var(--r-sm);
    background: rgba(255, 255, 255, 0.035);
    box-shadow: inset 0 0 0 1px var(--line-soft);
    overflow: hidden;
  }
  .cell:hover {
    background: rgba(255, 255, 255, 0.09);
  }
  .cell.filled {
    background: color-mix(in srgb, var(--c) 52%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--c) 70%, transparent);
  }
  .cell.filled:hover {
    background: color-mix(in srgb, var(--c) 68%, transparent);
  }
  .cell.live {
    box-shadow: inset 0 0 0 2px var(--ink);
  }

  .at {
    font: var(--t-caption);
    color: var(--ink-40);
  }
  .cell.filled .at {
    color: var(--ink);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
    margin-top: var(--sp-4);
    padding-top: var(--sp-3);
    border-top: 1px solid var(--line-soft);
  }

  .ghost {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px 11px;
    border-radius: var(--r-sm);
    font: var(--t-label);
    color: var(--ink-40);
    background: rgba(255, 255, 255, 0.05);
  }
  .ghost:hover:not(:disabled) {
    color: var(--danger);
    background: var(--danger-dim);
  }
  .ghost:disabled {
    opacity: 0.4;
  }
</style>
