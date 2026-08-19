<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';

  import { nuru } from '$lib/store.svelte';
  import { win } from '$lib/bridge';
  import Fader from './Fader.svelte';
  import Icon from './Icon.svelte';
  import NookView from './NookView.svelte';
  import NookRoom from './NookRoom.svelte';

  const SKY: Record<string, [string, string, string]> = {
    night: ['#080a14', '#131a2e', '#1b2136'],
    morning: ['#1a2436', '#4a5a6b', '#6b6553'],
    dusk: ['#0d1016', '#171c26', '#1e232c'],
  };

  const VIEW: Record<string, [string, string, string]> = {
    forest: ['#16281c', '#0f1f16', '#07110c'],
    city: ['#141c26', '#0d141d', '#070b10'],
    beach: ['#13303b', '#0d2531', '#071820'],
    hills: ['#151a20', '#0f1319', '#080a0d'],
  };

  const scene = $derived(nuru.scene);
  const sky = $derived(SKY[scene.sky] ?? SKY.dusk);
  const view = $derived(VIEW[scene.window] ?? VIEW.hills);

  const roomAccent = $derived.by(() => {
    let best: { c: string; w: number } | null = null;
    for (const l of nuru.layers) {
      const s = nuru.byId.get(l.soundId);
      if (!s) continue;
      if (!best || l.volume > best.w) best = { c: s.accent, w: l.volume };
    }
    return best?.c ?? 'var(--nuru)';
  });

  let canvas = $state<HTMLCanvasElement | null>(null);

  interface Particle {
    kind: 'rain' | 'snow';
    x: number;
    y: number;
    vy: number;
    vx: number;
    len: number;
    a: number;
  }

  $effect(() => {
    const el = canvas;
    const rain = scene.rain;
    const snow = scene.snow;
    const wind = scene.wind;
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

    function spawn(kind: 'rain' | 'snow', anywhere: boolean): Particle {
      const h = el ? el.height : 0;
      const w = el ? el.width : 0;
      const isSnow = kind === 'snow';
      const drive = 1 + wind * 1.4;
      return {
        kind,
        x: Math.random() * w * 1.3 - w * 0.15,
        y: anywhere ? Math.random() * h : -20 * dpr,
        vy: (isSnow ? (0.35 + Math.random() * 0.5) * (1 + wind * 0.5) : (7 + Math.random() * 6) * drive) * dpr,
        vx: (isSnow ? (Math.random() - 0.5) * 0.6 - wind * 2.2 : -0.9 - Math.random() * 0.5 - wind * 4.5) * dpr,
        len: (isSnow ? 1.6 + Math.random() * 1.8 : (9 + Math.random() * 13) * (1 + wind * 0.5)) * dpr,
        a: isSnow ? 0.35 + Math.random() * 0.45 : 0.1 + Math.random() * 0.22,
      };
    }

    function seed() {
      if (!el) return;
      const rainCount = Math.round(Math.min(rain, 1.6) * 260);
      const snowCount = Math.round(Math.min(snow, 1.6) * 150);
      particles = [
        ...Array.from({ length: rainCount }, () => spawn('rain', true)),
        ...Array.from({ length: snowCount }, () => spawn('snow', true)),
      ];
    }

    function frame() {
      if (!el || !ctx) return;
      ctx.clearRect(0, 0, el.width, el.height);

      for (const p of particles) {
        p.x += p.vx;
        p.y += p.vy;
        if (p.y > el.height + 20 * dpr || p.x < -60 * dpr || p.x > el.width + 60 * dpr) {
          Object.assign(p, spawn(p.kind, false));
        }

        if (p.kind === 'snow') {
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
  style:--wet={Math.min(1, scene.rain)}
  style:--gale={Math.min(1, scene.wind)}
  style:--fire={Math.min(1, scene.hearth)}
  transition:fade={{ duration: 320 }}
>
  <div class="sky"></div>

  <div class="outside">
    <NookView view={scene.window} />
  </div>

  {#if scene.storm > 0.05}
    <div class="lightning" style:--rate="{Math.max(3, 12 - scene.storm * 8)}s"></div>
  {/if}

  <canvas bind:this={canvas} class="weather"></canvas>

  <div class="haze" aria-hidden="true"></div>

  <div class="frame">
    <span class="mullion v"></span>
    <span class="mullion h"></span>
  </div>
  <div class="sill"></div>

  <NookRoom />

  {#if scene.hearth > 0.02}
    <div class="hearth" class:campfire={scene.hearthKind === 'campfire'}></div>
  {/if}

  <div class="vignette"></div>

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

    <button class="u-pressable round" onclick={exit} aria-label="Leave cozy mode" title="Esc">
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
  .lightning,
  .haze {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .sky {
    transition: background var(--dur-scene) var(--ease-in-out);
  }

  .sky {
    background: linear-gradient(180deg, var(--sky-0) 0%, var(--sky-1) 52%, var(--sky-2) 100%);
  }

  .weather {
    width: 100%;
    height: 100%;
  }

  .haze {
    background: linear-gradient(180deg, rgba(150, 170, 200, 0.14), rgba(120, 140, 170, 0.05));
    opacity: calc(var(--wet) * 0.85);
    transition: opacity var(--dur-scene) var(--ease-in-out);
  }

  .lightning {
    background: rgba(215, 228, 255, 0.9);
    opacity: 0;
    animation: flash var(--rate, 9s) steps(1, end) infinite;
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

  .hearth {
    background: radial-gradient(
      120% 60% at 22% 108%,
      color-mix(in srgb, #ff8a3d 55%, transparent) 0%,
      transparent 62%
    );
    mix-blend-mode: screen;
    opacity: var(--fire);
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
      filter: brightness(0.9);
    }
    28% {
      filter: brightness(1.2);
    }
    54% {
      filter: brightness(0.8);
    }
    76% {
      filter: brightness(1.1);
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
