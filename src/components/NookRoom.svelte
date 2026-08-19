<script lang="ts">
  const W = 1600;
  const H = 900;
  const DESK = 742;

  function rng(seed: number) {
    let s = seed >>> 0;
    return () => {
      s = (s * 1664525 + 1013904223) >>> 0;
      return s / 4294967296;
    };
  }

  const shelf = $derived.by(() => {
    const x0 = 26;
    const x1 = 250;
    const boards = [188, 330, 470, 604];
    const frame =
      `M${x0} 0h${x1 - x0}v${DESK}h-14V14h-${x1 - x0 - 28}v${DESK - 14}h-14Z` +
      boards.map((y) => `M${x0} ${y}h${x1 - x0}v13h-${x1 - x0}Z`).join('');

    const r = rng(20260818);
    const items: string[] = [];
    boards.forEach((y, bi) => {
      let x = x0 + 20;
      const limit = x1 - 22;
      let guard = 0;
      while (x < limit - 14 && guard++ < 30) {
        const kind = r();
        if (kind < 0.68) {
          const w = 11 + r() * 15;
          const h = 52 + r() * 46;
          const lean = bi === 2 && x > x0 + 120 && r() < 0.4;
          if (lean) {
            items.push(`M${x.toFixed(1)} ${y}l${(w * 1.7).toFixed(1)} -${(h * 0.5).toFixed(1)}l${w.toFixed(1)} ${(h * 0.24).toFixed(1)}l-${(w * 1.7).toFixed(1)} ${(h * 0.5).toFixed(1)}Z`);
            x += w * 2.2;
          } else {
            items.push(`M${x.toFixed(1)} ${(y - h).toFixed(1)}h${w.toFixed(1)}v${h.toFixed(1)}h-${w.toFixed(1)}Z`);
            x += w + 3;
          }
        } else if (kind < 0.86) {
          const w = 42 + r() * 30;
          const h = 34 + r() * 18;
          items.push(`M${x.toFixed(1)} ${(y - h).toFixed(1)}h${w.toFixed(1)}v${h.toFixed(1)}h-${w.toFixed(1)}Z`);
          items.push(`M${(x + 6).toFixed(1)} ${(y - h + 7).toFixed(1)}h${(w - 12).toFixed(1)}v4h-${(w - 12).toFixed(1)}Z`);
          x += w + 8;
        } else {
          const rr = 15 + r() * 9;
          items.push(
            `M${(x + rr).toFixed(1)} ${y}a${rr.toFixed(1)} ${rr.toFixed(1)} 0 1 1 0 -0.1Z` +
              `M${(x + rr * 0.55).toFixed(1)} ${(y - rr * 1.9).toFixed(1)}h${(rr * 0.9).toFixed(1)}v${(rr * 0.9).toFixed(1)}h-${(rr * 0.9).toFixed(1)}Z`,
          );
          x += rr * 2 + 10;
        }
      }
    });
    return { frame, items: items.join('') };
  });

  const hanger = $derived.by(() => {
    const x = 366;
    const potTop = 178;
    const pot = `M${x - 46} ${potTop}h92l-16 62h-60Z`;
    const cords = `M${x - 40} ${potTop}L${x - 12} 0M${x + 40} ${potTop}L${x + 12} 0`;
    const vines: string[] = [];
    const r = rng(4471);
    for (let i = 0; i < 7; i++) {
      const sx = x - 38 + i * 12.6;
      const len = 90 + r() * 210;
      const sway = (r() - 0.5) * 70;
      vines.push(`M${sx.toFixed(1)} ${potTop + 30}q${sway.toFixed(1)} ${(len * 0.55).toFixed(1)} ${(sway * 0.4).toFixed(1)} ${len.toFixed(1)}`);
      for (let l = 1; l < 5; l++) {
        const t = l / 5;
        const lx = sx + sway * t * 0.8;
        const ly = potTop + 30 + len * t;
        vines.push(`M${lx.toFixed(1)} ${ly.toFixed(1)}q10 -9 19 -2q-9 9 -19 2Z`);
      }
    }
    return { pot, cords, vines: vines.join('') };
  });

  const cat = $derived.by(() => {
    const x = 900;
    const y = DESK;
    const body = `M${x - 108} ${y}q6 -74 74 -76q74 -2 92 52q6 18 34 22q16 2 22 -2v10q-30 6 -44 -4q-16 -12 -22 -26q-18 -44 -80 -42q-62 2 -66 66Z`;
    const fill = `M${x - 108} ${y}q4 -70 72 -72q70 -2 88 50q8 22 40 24v8h-200Z`;
    const head = `M${x - 112} ${y - 52}q-10 -34 18 -44q2 -24 14 -6q16 -6 30 4q14 10 12 28q-4 26 -34 28q-32 2 -40 -10Z`;
    const ears = `M${x - 122} ${y - 88}l2 -26 20 16ZM${x - 66} ${y - 96}l16 -22 6 26Z`;
    const tail = `M${x + 76} ${y}q52 -4 66 -34q6 -12 -6 -15q-10 -3 -14 9q-9 20 -46 24Z`;
    return { body, fill, head, ears, tail };
  });

  const clutter = $derived.by(() => {
    const cup = `M712 ${DESK}h58l-7 -70h-44Z`;
    const pens =
      `M722 ${DESK - 70}l-8 -54h9l10 54ZM736 ${DESK - 70}l2 -66h9l-2 66ZM752 ${DESK - 70}l12 -48h9l-12 48Z`;
    const mug = `M604 ${DESK}h62l-6 -50h-50Z`;
    const handle = `M666 ${DESK - 44}q22 2 22 15q0 13 -20 15v-7q12 -1 12 -8q0 -7 -14 -8Z`;
    const steam = `M618 ${DESK - 60}q-12 -18 2 -32q11 -13 2 -26M642 ${DESK - 58}q-10 -15 2 -28q10 -11 2 -22`;
    const stack = `M300 ${DESK}h150v-16h-150ZM310 ${DESK - 16}h134v-14h-134ZM296 ${DESK - 30}h142v-13h-142Z`;
    const bottle = `M1512 ${DESK}h44v-72h-12v-26h-20v26h-12Z`;
    return { cup, pens, mug, handle, steam, stack, bottle };
  });

</script>

<svg class="room" viewBox="0 0 {W} {H}" preserveAspectRatio="xMidYMax slice" aria-hidden="true">
  <defs>
  </defs>

  <g class="ink">
    <path d={shelf.frame} />
    <path d={shelf.items} />

    <path d={hanger.pot} />
    <path d={hanger.vines} />

    <path d={clutter.stack} />
    <path d={cat.body} />
    <path d={cat.fill} />
    <path d={cat.head} />
    <path d={cat.ears} />
    <path d={cat.tail} />

    <path d="M-40 {DESK}h{W + 80}v{H - DESK + 40}h-{W + 80}Z" />

    <path d={clutter.cup} />
    <path d={clutter.pens} />
    <path d={clutter.mug} />
    <path d={clutter.handle} />
    <path d={clutter.bottle} />
  </g>

  <path class="cord" d={hanger.cords} />
  <path class="steam" d={clutter.steam} />
</svg>

<style>
  .room {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 1;
  }

  .ink path {
    fill: #05060a;
  }

  .cord {
    fill: none;
    stroke: #05060a;
    stroke-width: 4;
  }

  .steam {
    fill: none;
    stroke: rgba(232, 216, 196, 0.14);
    stroke-width: 4;
    stroke-linecap: round;
  }
</style>
