<script lang="ts">
  let { view = 'hills' }: { view?: string } = $props();

  const W = 1600;
  const H = 900;

  function rng(seed: number) {
    let s = seed >>> 0;
    return () => {
      s = (s * 1664525 + 1013904223) >>> 0;
      return s / 4294967296;
    };
  }

  function pine(x: number, y: number, w: number, h: number) {
    const t = (f: number) => (y - h * f).toFixed(1);
    const o = (f: number) => (x + w * f).toFixed(1);
    return [
      `M${x.toFixed(1)} ${(y - h).toFixed(1)}`,
      `L${o(0.26)} ${t(0.58)}`,
      `L${o(0.14)} ${t(0.58)}`,
      `L${o(0.4)} ${t(0.26)}`,
      `L${o(0.22)} ${t(0.26)}`,
      `L${o(0.5)} ${y.toFixed(1)}`,
      `L${o(-0.5)} ${y.toFixed(1)}`,
      `L${o(-0.22)} ${t(0.26)}`,
      `L${o(-0.4)} ${t(0.26)}`,
      `L${o(-0.14)} ${t(0.58)}`,
      `L${o(-0.26)} ${t(0.58)}`,
      'Z',
    ].join('');
  }

  function broadleaf(x: number, y: number, w: number, h: number) {
    const r = w / 2;
    const cy = y - h + r * 0.85;
    return (
      `M${(x - w * 0.045).toFixed(1)} ${y.toFixed(1)}` +
      `L${(x - w * 0.03).toFixed(1)} ${(cy + r * 0.5).toFixed(1)}` +
      `a${r.toFixed(1)} ${(r * 0.86).toFixed(1)} 0 1 1 ${(w * 0.06).toFixed(1)} 0` +
      `L${(x + w * 0.045).toFixed(1)} ${y.toFixed(1)}Z`
    );
  }

  const treeline = $derived.by(() => {
    if (view !== 'forest') return [];
    const bands = [
      { y: 640, count: 46, h: [70, 130], w: [46, 78], depth: 0 },
      { y: 730, count: 32, h: [110, 200], w: [66, 108], depth: 1 },
      { y: 900, count: 20, h: [190, 330], w: [104, 168], depth: 2 },
    ];
    return bands.map((b) => {
      const r = rng(1337 + b.depth * 977);
      const paths: string[] = [];
      for (let i = 0; i < b.count; i++) {
        const x = -80 + ((W + 160) / (b.count - 1)) * i + (r() - 0.5) * 44;
        const h = b.h[0] + r() * (b.h[1] - b.h[0]);
        const w = b.w[0] + r() * (b.w[1] - b.w[0]);
        paths.push(pine(x, b.y + r() * 12, w, h));
      }
      return { depth: b.depth, d: paths.join('') };
    });
  });

  const skyline = $derived.by(() => {
    if (view !== 'city') return [];
    const bands = [
      { y: 660, count: 26, h: [90, 210], w: [40, 92], depth: 0 },
      { y: 760, count: 18, h: [150, 340], w: [64, 132], depth: 1 },
      { y: 900, count: 11, h: [230, 430], w: [110, 210], depth: 2 },
    ];
    return bands.map((b) => {
      const r = rng(4242 + b.depth * 613);
      const blocks: Array<{ x: number; y: number; w: number; h: number; win: string }> = [];
      let x = -70;
      for (let i = 0; i < b.count; i++) {
        const w = b.w[0] + r() * (b.w[1] - b.w[0]);
        const h = b.h[0] + r() * (b.h[1] - b.h[0]);
        const top = b.y - h;
        const cols = Math.max(2, Math.floor(w / 22));
        const rows = Math.max(3, Math.floor(h / 30));
        const win: string[] = [];
        if (b.depth > 0) {
          for (let c = 0; c < cols; c++) {
            for (let q = 0; q < rows; q++) {
              if (r() < 0.42) continue;
              const wx = x + 8 + (c * (w - 16)) / cols;
              const wy = top + 14 + (q * (h - 26)) / rows;
              win.push(`M${wx.toFixed(1)} ${wy.toFixed(1)}h7v10h-7Z`);
            }
          }
        }
        blocks.push({ x, y: top, w, h, win: win.join('') });
        x += w + 4 + r() * 16;
      }
      return { depth: b.depth, blocks };
    });
  });

  const shore = $derived.by(() => {
    if (view !== 'beach') return null;
    const r = rng(90210);
    const waves: string[] = [];
    for (let i = 0; i < 16; i++) {
      const y = 690 + i * 13;
      const amp = 5 + i * 0.9;
      const seg = 150 + r() * 90;
      let d = `M-40 ${y.toFixed(1)}`;
      for (let x = -40; x < W + 80; x += seg) {
        d += `q${(seg / 2).toFixed(1)} ${(-amp).toFixed(1)} ${seg.toFixed(1)} 0`;
      }
      waves.push(d);
    }
    const palms: string[] = [];
    for (const [px, ph, dir] of [
      [190, 300, 1],
      [1430, 250, -1],
    ] as Array<[number, number, number]>) {
      const base = 900;
      const topY = base - ph;
      palms.push(
        `M${px - 9} ${base}q${6 * dir} ${-ph * 0.55} ${20 * dir} ${-ph}l10 2q${-12 * dir} ${ph * 0.56} ${-16 * dir} ${ph}Z`,
      );
      for (let f = 0; f < 6; f++) {
        const a = -0.35 + f * 0.42;
        const lx = px + 20 * dir;
        const ex = lx + Math.cos(a) * 118 * dir;
        const ey = topY + Math.sin(a) * 96 - 10;
        palms.push(
          `M${lx} ${topY}q${(ex - lx) * 0.55} ${(ey - topY) * 0.2} ${ex - lx} ${ey - topY}q${(lx - ex) * 0.4} ${(topY - ey) * 0.55} ${lx - ex} ${topY - ey}Z`,
        );
      }
    }
    return { waves: waves.join(''), palms: palms.join('') };
  });

  const downs = $derived.by(() => {
    if (view !== 'hills' && view !== 'forest' && view !== 'city' && view !== 'beach') return null;
    if (view !== 'hills') return null;
    const layer = (y: number, amp: number, seg: number, seed: number) => {
      const r = rng(seed);
      let d = `M-60 ${H}L-60 ${y.toFixed(1)}`;
      for (let x = -60; x < W + 120; x += seg) {
        d += `q${(seg / 2).toFixed(1)} ${(-amp - r() * amp).toFixed(1)} ${seg.toFixed(1)} ${((r() - 0.5) * amp * 0.5).toFixed(1)}`;
      }
      d += `L${W + 120} ${H}Z`;
      return d;
    };
    const posts: string[] = [];
    for (let i = 0; i < 14; i++) {
      const x = 60 + i * 118;
      const y = 858 - Math.sin(i * 0.7) * 10;
      posts.push(`M${x} ${y}h9v56h-9Z`);
    }
    posts.push(`M60 ${812}q560 26 1180 -6v12q-620 32 -1180 6Z`);
    return {
      far: layer(690, 40, 260, 7001),
      mid: layer(760, 54, 220, 7333),
      near: layer(846, 30, 300, 7777),
      tree: broadleaf(1240, 848, 210, 250),
      posts: posts.join(''),
    };
  });
</script>

<svg
  class="view"
  viewBox="0 0 {W} {H}"
  preserveAspectRatio="xMidYMax slice"
  aria-hidden="true"
>
  {#if view === 'forest'}
    {#each treeline as band (band.depth)}
      <path d={band.d} class="d{band.depth}" />
    {/each}
  {:else if view === 'city'}
    {#each skyline as band (band.depth)}
      <g class="d{band.depth}">
        {#each band.blocks as b, i (i)}
          <path d="M{b.x.toFixed(1)} {b.y.toFixed(1)}h{b.w.toFixed(1)}v{(H - b.y).toFixed(1)}h{(-b.w).toFixed(1)}Z" />
        {/each}
      </g>
      {#if band.depth > 0}
        <g class="win">
          {#each band.blocks as b, i (i)}
            {#if b.win}
              <path d={b.win} />
            {/if}
          {/each}
        </g>
      {/if}
    {/each}
  {:else if view === 'beach' && shore}
    <path class="d0" d="M-60 690H{W + 120}V{H}H-60Z" />
    <path class="ripple" d={shore.waves} />
    <path class="d2" d="M-60 862q400 -26 820 -4t840 -10V{H}H-60Z" />
    <path class="d2" d={shore.palms} />
  {:else if downs}
    <path class="d0" d={downs.far} />
    <path class="d1" d={downs.mid} />
    <path class="d2" d={downs.near} />
    <path class="d2" d={downs.posts} />
    <path class="d2" d={downs.tree} />
  {/if}
</svg>

<style>
  .view {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .d0 {
    fill: var(--view-0);
    filter: blur(1px);
  }
  .d1 {
    fill: var(--view-1);
  }
  .d2 {
    fill: var(--view-2);
  }

  .win path {
    fill: #ffd79a;
    opacity: calc(0.55 - var(--wet, 0) * 0.2);
  }

  .ripple {
    fill: none;
    stroke: rgba(190, 225, 255, 0.16);
    stroke-width: 3;
  }
</style>
