<script lang="ts">
  import { nuru } from '$lib/store.svelte';
  import { api, IS_PREVIEW } from '$lib/bridge';
  import { VERSION, CHANNEL } from '$lib/version';
  import Panel from './Panel.svelte';
  import Icon from './Icon.svelte';

  const THEMES = [
    { id: 'midnight', name: 'Midnight', swatch: ['#131417', '#ffb454'] },
    { id: 'ember', name: 'Ember', swatch: ['#141110', '#ff9d4d'] },
    { id: 'slate', name: 'Slate', swatch: ['#15161c', '#7dd3fc'] },
  ];

  const SIZES = [
    { id: 'compact', name: 'Compact', preview: 14 },
    { id: 'small', name: 'Small', preview: 18 },
    { id: 'normal', name: 'Normal', preview: 23 },
    { id: 'large', name: 'Large', preview: 29 },
  ] as const;

  let unshippable = $state<string[]>([]);
  $effect(() => {
    void api.unshippableSounds().then((ids) => (unshippable = ids));
  });
</script>

<Panel title="Settings" onclose={() => (nuru.activePanel = 'none')} width={400}>
  <section>
    <h3>Theme</h3>
    <div class="themes">
      {#each THEMES as t (t.id)}
        <button
          class="u-pressable theme"
          class:on={nuru.theme === t.id}
          onclick={() => nuru.setTheme(t.id)}
        >
          <span
            class="chip"
            style:background="linear-gradient(135deg, {t.swatch[0]} 55%, {t.swatch[1]} 55%)"
          ></span>
          <span>{t.name}</span>
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h3>Tile size</h3>
    <div class="sizes">
      {#each SIZES as s (s.id)}
        <button
          class="u-pressable size"
          class:on={nuru.tileSize === s.id}
          onclick={() => nuru.setTileSize(s.id)}
        >
          <span class="box" style:--d="{s.preview}px"></span>
          <span>{s.name}</span>
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h3>Output device</h3>
    <div class="select">
      <select
        value={nuru.outputDevice ?? ''}
        onchange={(e) => {
          const v = (e.currentTarget as HTMLSelectElement).value;
          void nuru.setOutputDevice(v === '' ? null : v);
        }}
      >
        <option value="">System default</option>
        {#each nuru.outputDevices as d (d)}
          <option value={d}>{d}</option>
        {/each}
      </select>
    </div>
    <p class="note">
      Currently playing through {nuru.activeDevice || 'the system default'}. The choice is
      remembered, and falls back to the default if the device is unplugged.
    </p>
  </section>

  <section>
    <h3>Behaviour</h3>
    <label class="toggle">
      <input type="checkbox" bind:checked={nuru.restoreOnLaunch} />
      <span class="track"><span class="knob"></span></span>
      <span class="text">
        Restore the last mix on launch
        <em>Loads the sounds you had going, paused.</em>
      </span>
    </label>
  </section>

  <section>
    <h3>Audio</h3>
    <p class="line">
      <span class="k">Output</span>
      <span class="v">{nuru.engineNote ?? '-'}</span>
    </p>
    <p class="note">
      Every sound is decoded from a local lossless file and looped at an exact
      sample boundary. Nothing streams from the network while it plays.
    </p>
  </section>

  {#if unshippable.length || IS_PREVIEW}
    <section class="warn">
      <h3><Icon name="warn" size={14} /> Development build</h3>
      {#if unshippable.length}
        <p class="note">
          {unshippable.length} loaded {unshippable.length === 1 ? 'sound is' : 'sounds are'}
          development placeholder{unshippable.length === 1 ? '' : 's'} taken from Elpy.
          They are reference material and must be replaced before Nuru is released.
        </p>
      {/if}
      {#if IS_PREVIEW}
        <p class="note">
          Running in a browser, so the Rust audio engine is not attached - the
          interface is live but silent.
        </p>
      {/if}
    </section>
  {/if}

  <footer>
    <span class="u-numeric">Nuru {VERSION}</span>
    <span class="channel">{CHANNEL}</span>
  </footer>
</Panel>

<style>
  section {
    padding-bottom: var(--sp-5);
    margin-bottom: var(--sp-5);
    border-bottom: 1px solid var(--line-soft);
  }
  section:last-of-type {
    border-bottom: none;
    margin-bottom: var(--sp-3);
  }

  h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    font: var(--t-caption);
    letter-spacing: var(--tracking-caps);
    text-transform: uppercase;
    color: var(--ink-40);
    margin-bottom: var(--sp-3);
  }

  .themes {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--sp-2);
  }

  .theme {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-3) var(--sp-2);
    border-radius: var(--r-md);
    font: var(--t-caption);
    color: var(--ink-60);
    box-shadow: inset 0 0 0 1px var(--line-soft);
  }
  .theme:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--ink);
  }
  .theme.on {
    color: var(--ink);
    box-shadow: inset 0 0 0 1.5px var(--nuru);
    background: var(--nuru-ghost);
  }

  .chip {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .sizes {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--sp-2);
  }

  .size {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: var(--sp-2);
    height: 74px;
    padding: var(--sp-2);
    border-radius: var(--r-md);
    font: var(--t-caption);
    color: var(--ink-60);
    box-shadow: inset 0 0 0 1px var(--line-soft);
  }
  .size:hover,
  .size:focus-visible {
    background: rgba(255, 255, 255, 0.04);
    color: var(--ink);
  }
  .size.on {
    color: var(--ink);
    background: var(--nuru-ghost);
    box-shadow: inset 0 0 0 1.5px var(--nuru);
  }

  .box {
    width: var(--d);
    height: var(--d);
    border-radius: 4px;
    background: var(--ink-25);
  }
  .size.on .box {
    background: var(--nuru);
  }

  .select {
    position: relative;
  }
  .select select {
    width: 100%;
    padding: 9px 11px;
    border-radius: var(--r-sm);
    background: rgba(255, 255, 255, 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink);
    font: var(--t-body);
    cursor: default;
  }
  .select select:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .select option {
    background: var(--s-700);
    color: var(--ink);
  }

  .toggle {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-3);
    cursor: default;
  }
  .toggle input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .track {
    flex: 0 0 auto;
    width: 34px;
    height: 20px;
    margin-top: 1px;
    border-radius: var(--r-pill);
    background: rgba(255, 255, 255, 0.1);
    padding: 3px;
    transition: background-color var(--dur-2) var(--ease-out);
  }

  .knob {
    display: block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--ink-60);
    transition:
      transform var(--dur-3) var(--ease-spring),
      background-color var(--dur-2) var(--ease-out);
  }

  .toggle input:checked + .track {
    background: var(--nuru);
  }
  .toggle input:checked + .track .knob {
    transform: translateX(14px);
    background: var(--ink-inverse);
  }

  .toggle input:focus-visible + .track {
    background: rgba(255, 255, 255, 0.2);
  }
  .toggle input:checked:focus-visible + .track {
    background: var(--nuru-bright);
  }

  .text {
    font: var(--t-body);
    color: var(--ink-80);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .text em {
    font: var(--t-caption);
    font-style: normal;
    color: var(--ink-40);
  }

  .line {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-4);
    font: var(--t-body);
    margin-bottom: var(--sp-2);
  }
  .k {
    color: var(--ink-40);
  }
  .v {
    color: var(--ink-80);
    text-align: right;
  }

  .note {
    font: var(--t-caption);
    color: var(--ink-40);
    line-height: 1.65;
  }

  .warn h3 {
    color: var(--nuru);
  }
  .warn {
    padding: var(--sp-4);
    border-radius: var(--r-md);
    background: var(--nuru-ghost);
    border-bottom: none;
  }
  .warn .note + .note {
    margin-top: var(--sp-2);
  }

  footer {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font: var(--t-caption);
    color: var(--ink-25);
  }

  .channel {
    text-transform: uppercase;
    letter-spacing: var(--tracking-caps);
    padding: 1px 6px;
    border-radius: var(--r-pill);
    background: rgba(255, 255, 255, 0.06);
  }
</style>
