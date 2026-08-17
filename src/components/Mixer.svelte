<script lang="ts">
  import { flip } from 'svelte/animate';
  import { fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  import { nuru } from '$lib/store.svelte';
  import Fader from './Fader.svelte';
  import Icon from './Icon.svelte';

  const canPlay = $derived(nuru.layers.length > 0);

  // `animate:` needs the animated element to be the only child of its keyed
  // block, so the layer/sound pairing is resolved here rather than with an
  // {#if} inside the loop.
  const rows = $derived(
    nuru.layers
      .map((layer) => ({ layer, sound: nuru.byId.get(layer.soundId) }))
      .filter((r): r is { layer: (typeof nuru.layers)[number]; sound: NonNullable<typeof r.sound> } =>
        Boolean(r.sound),
      ),
  );
</script>

<aside class="mixer">
  <header>
    <h2>Mixer</h2>
    {#if nuru.layers.length}
      <button
        class="u-pressable clear"
        onclick={() => nuru.clear()}
        title="Stop everything"
        aria-label="Stop everything"
      >
        <Icon name="trash" size={15} />
      </button>
    {/if}
  </header>

  <div class="master">
    <div class="row">
      <span class="name">Master</span>
      <span class="u-numeric value">{Math.round(nuru.masterVolume * 100)}</span>
    </div>
    <Fader
      label="Master volume"
      value={nuru.masterVolume}
      onchange={(v) => nuru.setMaster(v)}
    />
  </div>

  <div class="layers" role="list">
    {#each rows as { layer, sound } (layer.soundId)}
      <div
        class="layer"
        role="listitem"
        animate:flip={{ duration: 260, easing: cubicOut }}
        in:fly={{ y: -10, duration: 240, easing: cubicOut }}
        out:fly={{ x: -18, duration: 180, easing: cubicOut }}
        style:--accent={sound.accent}
      >
        <div class="row">
          <span class="dot" aria-hidden="true"></span>
          <span class="name">{sound.name}</span>
          <span class="u-numeric value">{Math.round(layer.volume * 100)}</span>
          <button
            class="u-pressable remove"
            onclick={() => nuru.remove(layer.soundId)}
            title="Remove {sound.name}"
            aria-label="Remove {sound.name}"
          >
            <Icon name="close" size={13} stroke={2} />
          </button>
        </div>
        <Fader
          label="{sound.name} volume"
          value={layer.volume}
          accent={sound.accent}
          onchange={(v) => nuru.setVolume(layer.soundId, v)}
        />
      </div>
    {/each}

    {#if !nuru.layers.length}
      <p class="empty" in:fly={{ y: 8, duration: 260, easing: cubicOut }}>
        Pick a sound to start. Layer as many as you like — they all loop and mix
        together.
      </p>
    {/if}
  </div>

  <footer>
    <button
      class="u-pressable transport"
      class:playing={nuru.playing && canPlay}
      disabled={!canPlay}
      onclick={() => nuru.togglePlaying()}
      title={nuru.playing ? 'Pause (Space)' : 'Play (Space)'}
      aria-label={nuru.playing ? 'Pause' : 'Play'}
    >
      <span class="glyph">
        {#if nuru.playing && canPlay}
          <Icon name="pause" size={20} stroke={2.2} />
        {:else}
          <Icon name="play" size={20} fill />
        {/if}
      </span>
    </button>

    <div class="actions">
      <button
        class="u-pressable ghost"
        disabled={!canPlay}
        onclick={() => (nuru.activePanel = 'presets')}
      >
        <Icon name="save" size={15} />
        <span>Save mix</span>
      </button>
      <button class="u-pressable ghost" onclick={() => (nuru.activePanel = 'presets')}>
        <Icon name="stack" size={15} />
        <span>Mixes</span>
      </button>
    </div>
  </footer>
</aside>

<style>
  .mixer {
    width: var(--mixer-w);
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--line);
    background: linear-gradient(180deg, var(--s-700), var(--s-800) 42%);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-5) var(--sp-5) var(--sp-3);
  }

  h2 {
    font: var(--t-title);
    letter-spacing: -0.01em;
  }

  .clear {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--ink-40);
  }
  .clear:hover {
    color: var(--danger);
    background: var(--danger-dim);
  }

  .master {
    padding: 0 var(--sp-5) var(--sp-4);
    border-bottom: 1px solid var(--line-soft);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin-bottom: var(--sp-2);
    min-height: 20px;
  }

  .name {
    font: var(--t-label);
    color: var(--ink-80);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .value {
    font-size: 11px;
    color: var(--ink-40);
    font-variant-numeric: tabular-nums;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 10px -1px var(--accent);
    flex: 0 0 auto;
  }

  .layers {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--sp-4) var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .layer .name {
    color: var(--ink);
  }

  .remove {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: var(--r-xs);
    color: var(--ink-25);
    opacity: 0;
    transition:
      opacity var(--dur-2) var(--ease-out),
      color var(--dur-2) var(--ease-out),
      background-color var(--dur-2) var(--ease-out);
  }
  .layer:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }
  .remove:hover {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.08);
  }

  .empty {
    font: var(--t-body);
    color: var(--ink-40);
    line-height: 1.6;
    padding: var(--sp-2) 0;
  }

  footer {
    padding: var(--sp-4) var(--sp-5) var(--sp-5);
    border-top: 1px solid var(--line-soft);
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .transport {
    width: 52px;
    height: 52px;
    flex: 0 0 auto;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--ink);
    color: var(--ink-inverse);
    box-shadow: var(--e-2);
  }

  /* A ring that only exists while sound is coming out. Cheaper to read at a
     glance than the play/pause glyph alone. */
  .transport.playing {
    box-shadow:
      var(--e-2),
      0 0 0 4px var(--nuru-ghost);
  }

  .transport:hover:not(:disabled) {
    background: #fff;
  }

  .glyph {
    display: grid;
    place-items: center;
    animation: glyph-in var(--dur-3) var(--ease-spring);
  }

  @keyframes glyph-in {
    from {
      opacity: 0;
      transform: scale(0.6);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    flex: 1;
    min-width: 0;
  }

  .ghost {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: 7px 10px;
    border-radius: var(--r-sm);
    color: var(--ink-60);
    background: rgba(255, 255, 255, 0.04);
    white-space: nowrap;
  }
  .ghost:hover:not(:disabled),
  .ghost:focus-visible:not(:disabled) {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.09);
  }
</style>
