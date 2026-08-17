<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  import { nuru } from '$lib/store.svelte';
  import { win } from '$lib/bridge';
  import Fader from './Fader.svelte';
  import Icon from './Icon.svelte';

  /**
   * Nook Mode — a full-screen room that reacts to what is playing.
   *
   * Built entirely from gradients and a particle canvas, deliberately: it is a
   * *structure* for real artwork, not a substitute for it. Every visual is
   * driven by `nuru.scene`, which resolves one winning state per channel, so
   * dropping in painted layers later means replacing the paint, not rewiring the
   * logic. Adding a sound changes the room within one cross-dissolve.
   */

  // Palettes per sky state: [zenith, horizon, ground haze]
  const SKY: Record<string, [string, string, string]> = {
    night: ['#080a14', '#131a2e', '#1b2136'],
    morning: ['#1a2436', '#4a5a6b', '#6b6553'],
    default: ['#0d1016', '#171c26', '#1e232c'],
  };

  // Palettes per window state: [far, mid, near silhouette]
  const VIEW: Record<string, [string, string, string]> = {
    forest: ['#16281c', '#0f1f16', '#07110c'],
    city: ['#141c26', '#0d141d', '#070b10'],
    beach: ['#13303b', '#0d2531', '#071820'],
    default: ['#151a20', '#0f1319', '#080a0d'],
  };

  const sky = $derived(SKY[nuru.scene.sky ?? ''] ?? SKY.default);
  const view = $derived(VIEW[nuru.scene.window ?? ''] ?? VIEW.default);
  const weather = $derived(nuru.scene.weather ?? 'clear');
  const hearth = $derived(nuru.scene.hearth ?? null);

  /** Overall accent of the room — the strongest playing layer's colour. */
  const roomAccent = $derived.by(() => {
    let best: { c: string; w: number } | null = null;
    for (const l of nuru.layers) {
      const s = nuru.byId.get(l.soundId);
      if (!s) continue;
      if (!best || l.volume > best.w) best = { c: s.accent, w: l.volume };
    }
    return best?.c ?? 'var(--nuru)';
  });

  /* ── Particles ─────────────────────────────────────────────────────────────
     Rain and snow are a canvas rather than DOM nodes: a few hundred elements
     animating at once is exactly the kind of thing that turns a calm app into a
     fan-spinner, and the brief asks for the opposite. */

  let canvas = $state<HTMLCanvasElement | null>(null);

  interface Particle {
    x: number;
    y: number;
    vy: number;
    vx: number;
    len: number;
    a: number;
  }

  $effect(() => {
    const el = canvas;
    // Read reactive inputs here so the effect re-runs when the weather changes.
    const kind = weather;
    if (!el) return;

    const ctx = el.getContext('2d');
    if (!ctx) return;

    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    let raf = 0;
    let particles: Particle[] = [];
    let dpr = 1;

    function resize() {
      if (!el) return;
      dpr = Math.min(window.devicePixelRatio || 1, 2);
      el.width = Math.floor(el.clientWidth * dpr);
      el.height = Math.floor(el.clientHeight * dpr);
    }

    function seed() {
      if (!el) return;
      const density =
        kind === 'downpour' ? 420 : kind === 'rain' ? 210 : kind === 'storm' ? 340 : kind === 'snow' ? 180 : 0;
      particles = Array.from({ length: density }, () => spawn(true));
    }

    function spawn(anywhere: boolean): Particle {
      const h = el ? el.height : 0;
      const w = el ? el.width : 0;
      const snow = kind === 'snow';
      return {
        x: Math.random() * w,
        y: anywhere ? Math.random() * h : -20 * dpr,
        vy: (snow ? 0.35 + Math.random() * 0.5 : 7 + Math.random() * 6) * dpr,
        vx: (snow ? (Math.random() - 0.5) * 0.6 : -0.9 - Math.random() * 0.5) * dpr,
        len: (snow ? 1.6 + Math.random() * 1.8 : 9 + Math.random() * 13) * dpr,
        a: snow ? 0.35 + Math.random() * 0.45 : 0.1 + Math.random() * 0.22,
      };
    }

    function frame() {
      if (!el || !ctx) return;
      ctx.clearRect(0, 0, el.width, el.height);
      const snow = kind === 'snow';

      for (const p of particles) {
        p.x += p.vx;
        p.y += p.vy;
        if (p.y > el.height + 20 * dpr || p.x < -20 * dpr) Object.assign(p, spawn(false));

        if (snow) {
          ctx.beginPath();
          ctx.arc(p.x, p.y, p.len, 0, Math.PI * 2);
          ctx.fillStyle = `rgba(233,240,255,${p.a})`;
          ctx.fill();
        } else {
          ctx.beginPath();
          ctx.moveTo(p.x, p.y);
          ctx.lineTo(p.x - p.vx * 1.6, p.y - p.len);
          ctx.strokeStyle = `rgba(198,220,255,${p.a})`;
          ctx.lineWidth = 1.1 * dpr;
          ctx.stroke();
        }
      }
      raf = requestAnimationFrame(frame);
    }

    resize();
    seed();
    if (particles.length && !reduced) raf = requestAnimationFrame(frame);
    else if (ctx) ctx.clearRect(0, 0, el.width, el.height);

    const onResize = () => {
      resize();
      seed();
    };
    window.addEventListener('resize', onResize);

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener('resize', onResize);
    };
  });

  async function exit() {
    nuru.nookMode = false;
    await win.setFullscreen(false);
  }

  $effect(() => {
    if (nuru.nookMode) void win.setFullscreen(true);
  });

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape') void exit();
  }
</script>

<svelte:window onkeydown={keydown} />

<div
  class="nook"
  style:--sky-0={sky[0]}
  style:--sky-1={sky[1]}
  style:--sky-2={sky[2]}
  style:--view-0={view[0]}
  style:--view-1={view[1]}
  style:--view-2={view[2]}
  style:--room={roomAccent}
  transition:fade={{ duration: 320 }}
>
  <div class="sky"></div>

  <!-- The view through the window. Three bands of silhouette give parallax
       depth without any artwork; real layers slot in here. -->
  <div class="outside">
    <div class="band far"></div>
    <div class="band mid"></div>
    <div class="band near"></div>
  </div>

  {#if weather === 'storm'}
    <div class="lightning"></div>
  {/if}

  <canvas bind:this={canvas} class="weather"></canvas>

  <!-- Window frame and interior, drawn over the view. -->
  <div class="frame">
    <span class="mullion v"></span>
    <span class="mullion h"></span>
  </div>
  <div class="sill"></div>

  {#if hearth}
    <div class="hearth" class:campfire={hearth === 'campfire'}></div>
  {/if}

  <div class="vignette"></div>

  <!-- Controls fade away until the pointer comes near them. -->
  <div class="controls" transition:fly={{ y: 20, duration: 320, easing: cubicOut }}>
    <button
      class="u-pressable round"
      onclick={() => nuru.togglePlaying()}
      aria-label={nuru.playing ? 'Pause' : 'Play'}
    >
      {#if nuru.playing && nuru.layers.length}
        <Icon name="pause" size={18} stroke={2.2} />
      {:else}
        <Icon name="play" size={18} fill />
      {/if}
    </button>

    <div class="faders">
      {#each nuru.layers as layer (layer.soundId)}
        {@const sound = nuru.byId.get(layer.soundId)}
        {#if sound}
          <div class="strip">
            <span class="nm">{sound.name}</span>
            <Fader
              label="{sound.name} volume"
              value={layer.volume}
              accent={sound.accent}
              onchange={(v) => nuru.setVolume(layer.soundId, v)}
            />
          </div>
        {/if}
      {/each}
      {#if !nuru.layers.length}
        <span class="nothing">Nothing playing</span>
      {/if}
    </div>

    <button class="u-pressable round" onclick={exit} aria-label="Leave nook mode" title="Esc">
      <Icon name="collapse" size={17} />
    </button>
  </div>
</div>

<style>
  .nook {
    position: fixed;
    inset: 0;
    z-index: 100;
    overflow: hidden;
    background: #05060a;
  }

  .sky,
  .outside,
  .weather,
  .frame,
  .sill,
  .hearth,
  .vignette,
  .lightning {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  /* Every scene layer cross-dissolves on the same clock, so a sound switching on
     changes the whole room as one move rather than as a stack of separate
     transitions arriving at different times. */
  .sky,
  .band {
    transition: background var(--dur-scene) var(--ease-in-out);
  }

  .sky {
    background: linear-gradient(
      180deg,
      var(--sky-0) 0%,
      var(--sky-1) 52%,
      var(--sky-2) 100%
    );
  }

  .band {
    position: absolute;
    inset-inline: -5%;
    border-radius: 50% 50% 0 0 / 14% 14% 0 0;
  }
  .far {
    bottom: 26%;
    height: 26%;
    background: var(--view-0);
    filter: blur(1px);
  }
  .mid {
    bottom: 18%;
    height: 26%;
    background: var(--view-1);
  }
  .near {
    bottom: 0;
    height: 26%;
    background: var(--view-2);
  }

  .weather {
    width: 100%;
    height: 100%;
  }

  .lightning {
    background: rgba(215, 228, 255, 0.9);
    opacity: 0;
    animation: flash 9s steps(1, end) infinite;
    mix-blend-mode: screen;
  }
  @keyframes flash {
    0%,
    88%,
    100% {
      opacity: 0;
    }
    89% {
      opacity: 0.5;
    }
    90% {
      opacity: 0;
    }
    91.5% {
      opacity: 0.32;
    }
    93% {
      opacity: 0;
    }
  }

  /* The room is implied by a frame in front of the view rather than modelled —
     enough to place the viewer indoors. */
  .frame {
    box-shadow: inset 0 0 0 min(6vw, 76px) rgba(9, 10, 13, 0.94);
  }

  .mullion {
    position: absolute;
    background: rgba(9, 10, 13, 0.94);
  }
  .mullion.v {
    left: 50%;
    top: 0;
    bottom: 0;
    width: min(1.4vw, 18px);
    transform: translateX(-50%);
  }
  .mullion.h {
    top: 46%;
    left: 0;
    right: 0;
    height: min(1.4vw, 18px);
  }

  .sill {
    top: auto;
    height: 18vh;
    background: linear-gradient(180deg, rgba(9, 10, 13, 0) 0%, rgba(6, 7, 9, 0.96) 62%);
  }

  /* Firelight from below and to one side, breathing. */
  .hearth {
    background: radial-gradient(
      120% 60% at 22% 108%,
      color-mix(in srgb, #ff8a3d 55%, transparent) 0%,
      transparent 62%
    );
    mix-blend-mode: screen;
    animation: flicker 5.5s var(--ease-in-out) infinite;
  }
  .hearth.campfire {
    background: radial-gradient(
      100% 55% at 50% 112%,
      color-mix(in srgb, #ffa23d 62%, transparent) 0%,
      transparent 58%
    );
  }
  @keyframes flicker {
    0%,
    100% {
      opacity: 0.62;
    }
    28% {
      opacity: 0.86;
    }
    54% {
      opacity: 0.54;
    }
    76% {
      opacity: 0.78;
    }
  }

  .vignette {
    background: radial-gradient(120% 90% at 50% 42%, transparent 40%, rgba(0, 0, 0, 0.7) 100%);
  }

  .controls {
    position: absolute;
    left: 50%;
    bottom: 28px;
    transform: translateX(-50%);
    z-index: 2;
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    max-width: min(960px, calc(100vw - 48px));
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-2xl);
    background: rgba(12, 13, 16, 0.62);
    backdrop-filter: blur(20px) saturate(1.2);
    outline: 1px solid rgba(255, 255, 255, 0.08);
    outline-offset: -1px;
    box-shadow: var(--e-4);
    opacity: 0.28;
    transition: opacity var(--dur-4) var(--ease-out);
  }
  .controls:hover,
  .controls:focus-within {
    opacity: 1;
  }

  .round {
    width: 42px;
    height: 42px;
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--ink);
    background: rgba(255, 255, 255, 0.1);
  }
  .round:hover {
    background: rgba(255, 255, 255, 0.18);
  }

  .faders {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: var(--sp-5);
    overflow-x: auto;
    scrollbar-width: none;
    padding: 2px;
  }
  .faders::-webkit-scrollbar {
    display: none;
  }

  .strip {
    min-width: 132px;
    flex: 1 1 132px;
  }

  .nm {
    display: block;
    font: var(--t-caption);
    color: var(--ink-60);
    margin-bottom: 3px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .nothing {
    font: var(--t-caption);
    color: var(--ink-25);
    align-self: center;
    padding: 0 var(--sp-4);
  }

  @media (prefers-reduced-motion: reduce) {
    .lightning,
    .hearth {
      animation: none;
    }
  }
</style>
