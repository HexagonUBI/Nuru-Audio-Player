<script lang="ts">
  import { nuru } from '$lib/store.svelte';
  import Drawer from './Drawer.svelte';

  interface CreditGroup {
    packName: string;
    licence: string;
    attribution: string | null;
    shippable: boolean;
    names: string[];
  }

  const credits = $derived.by<CreditGroup[]>(() => {
    const groups = new Map<string, CreditGroup>();
    for (const sound of nuru.sounds) {
      const p = sound.provenance;
      const key = `${sound.packName}|${p.licence}|${p.attribution ?? ''}`;
      const found = groups.get(key);
      if (found) {
        found.names.push(sound.name);
        continue;
      }
      groups.set(key, {
        packName: sound.packName,
        licence: p.licence,
        attribution: p.attribution,
        shippable: p.shippable,
        names: [sound.name],
      });
    }
    return [...groups.values()].sort((a, b) => {
      if (a.shippable !== b.shippable) return a.shippable ? -1 : 1;
      return a.packName.localeCompare(b.packName);
    });
  });

  const packs = $derived(new Set(credits.map((c) => c.packName)).size);
</script>

<Drawer title="Credits" onclose={() => (nuru.activePanel = 'settings')} width={460}>
  <p class="lede">
    {nuru.sounds.length} sounds across {packs} {packs === 1 ? 'pack' : 'packs'}. Each pack
    carries its own licence and attribution, read from the pack itself rather than a list
    kept here.
  </p>

  {#each credits as g (g.packName + g.licence + (g.attribution ?? ''))}
    <div class="credit" class:unsafe={!g.shippable}>
      <p class="cpack">
        {g.packName}
        <span class="clic">{g.licence}</span>
      </p>
      {#if g.attribution}
        <p class="cattr">{g.attribution}</p>
      {/if}
      <p class="cnames">{g.names.join(', ')}</p>
      {#if !g.shippable}
        <p class="cwarn">Development material. Must not appear in a public build.</p>
      {/if}
    </div>
  {:else}
    <p class="lede">No sounds are loaded, so there is nothing to credit yet.</p>
  {/each}

  <button class="u-pressable back" onclick={() => (nuru.activePanel = 'settings')}>
    Back to settings
  </button>
</Drawer>

<style>
  .lede {
    font: var(--t-body);
    color: var(--ink-40);
    line-height: 1.65;
    margin-bottom: var(--sp-4);
  }

  .credit {
    padding: 12px 14px;
    margin-bottom: var(--sp-2);
    border-radius: var(--r-md);
    background: rgba(255, 255, 255, 0.04);
    box-shadow: inset 0 0 0 1px var(--line-soft);
  }
  .credit.unsafe {
    background: var(--danger-dim);
    box-shadow: inset 0 0 0 1px rgba(255, 95, 87, 0.3);
  }

  .cpack {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font: var(--t-label);
    color: var(--ink);
  }

  .clic {
    font: var(--t-caption);
    font-family: var(--font-numeric);
    color: var(--ink-40);
    padding: 1px 6px;
    border-radius: var(--r-pill);
    background: rgba(255, 255, 255, 0.06);
  }

  .cattr {
    margin-top: 4px;
    font: var(--t-body);
    color: var(--ink-80);
    line-height: 1.55;
  }

  .cnames {
    margin-top: 4px;
    font: var(--t-caption);
    color: var(--ink-40);
    line-height: 1.5;
  }

  .cwarn {
    margin-top: 6px;
    font: var(--t-caption);
    color: var(--danger);
  }

  .back {
    margin-top: var(--sp-5);
    width: 100%;
    padding: 10px;
    border-radius: var(--r-sm);
    font: var(--t-label);
    color: var(--ink-60);
    background: rgba(255, 255, 255, 0.05);
  }
  .back:hover {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.1);
  }
</style>
