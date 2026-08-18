<script lang="ts">
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  import { nuru } from '$lib/store.svelte';
  import Panel from './Panel.svelte';
  import Icon from './Icon.svelte';

  let name = $state('');

  const canSave = $derived(nuru.layers.length > 0 && name.trim().length > 0);

  function save() {
    if (!canSave) return;
    nuru.savePreset(name);
    name = '';
  }

  function swatches(ids: string[]) {
    return ids
      .slice(0, 4)
      .map((id) => nuru.byId.get(id)?.accent)
      .filter((c): c is string => Boolean(c));
  }
</script>

<Panel title="Mixes" onclose={() => (nuru.activePanel = 'none')} width={400}>
  <div class="save">
    <input
      type="text"
      placeholder={nuru.layers.length ? 'Name this mix' : 'Play something first'}
      bind:value={name}
      disabled={!nuru.layers.length}
      onkeydown={(e) => e.key === 'Enter' && save()}
      maxlength="48"
    />
    <button class="u-pressable solid" disabled={!canSave} onclick={save}>Save</button>
  </div>

  {#if nuru.layers.length}
    <p class="current">
      {nuru.layers.length}
      {nuru.layers.length === 1 ? 'sound' : 'sounds'} playing
    </p>
  {/if}

  <ul class="list">
    {#each nuru.presets as preset, i (preset.id)}
      <li in:fly={{ y: 8, duration: 220, delay: i * 24, easing: cubicOut }}>
        <button class="u-pressable item" onclick={() => nuru.loadPreset(preset.id)}>
          <span class="dots" aria-hidden="true">
            {#each swatches(preset.layers.map((l) => l.soundId)) as c, j (j)}
              <span class="dot" style:--c={c}></span>
            {/each}
          </span>
          <span class="meta">
            <span class="pname">{preset.name}</span>
            <span class="sub">
              {preset.layers.length}
              {preset.layers.length === 1 ? 'sound' : 'sounds'}
            </span>
          </span>
        </button>
        <button
          class="u-pressable del"
          onclick={() => nuru.deletePreset(preset.id)}
          aria-label="Delete {preset.name}"
          title="Delete"
        >
          <Icon name="trash" size={14} />
        </button>
      </li>
    {/each}
  </ul>

  {#if !nuru.presets.length}
    <p class="empty">
      No saved mixes yet. Get a combination you like, give it a name, and it will be
      here next time.
    </p>
  {/if}
</Panel>

<style>
  .save {
    display: flex;
    gap: var(--sp-2);
    margin-bottom: var(--sp-3);
  }

  .save input {
    flex: 1;
    min-width: 0;
    padding: 9px 11px;
    border-radius: var(--r-sm);
    background: rgba(255, 255, 255, 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    transition: box-shadow var(--dur-2) var(--ease-out);
  }
  .save input:focus {
    box-shadow: inset 0 0 0 1px var(--nuru);
  }
  .save input::placeholder {
    color: var(--ink-25);
  }
  .save input:disabled {
    opacity: 0.5;
  }

  .solid {
    padding: 9px 16px;
    border-radius: var(--r-sm);
    background: var(--ink);
    color: var(--ink-inverse);
    font-weight: 600;
  }
  .solid:hover:not(:disabled) {
    background: #fff;
  }
  .solid:disabled {
    opacity: 0.3;
  }

  .current {
    font: var(--t-caption);
    color: var(--ink-40);
    margin-bottom: var(--sp-4);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    border-radius: var(--r-md);
  }
  li:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .item {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: 10px var(--sp-3);
    border-radius: var(--r-md);
    text-align: left;
  }

  .dots {
    display: flex;
    flex: 0 0 auto;
  }

  .dot {
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: var(--c);
    box-shadow: 0 0 8px -1px var(--c);
    margin-left: -3px;
    outline: 2px solid var(--s-700);
  }
  .dot:first-child {
    margin-left: 0;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .pname {
    font: var(--t-label);
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    font: var(--t-caption);
    color: var(--ink-40);
  }

  .del {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--ink-25);
    opacity: 0;
    margin-right: 4px;
  }
  li:hover .del,
  .del:focus-visible {
    opacity: 1;
  }
  .del:hover {
    color: var(--danger);
    background: var(--danger-dim);
  }

  .empty {
    font: var(--t-body);
    color: var(--ink-40);
    line-height: 1.7;
    padding: var(--sp-4) 0 var(--sp-2);
  }
</style>
