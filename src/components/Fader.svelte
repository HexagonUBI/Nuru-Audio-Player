<script lang="ts">
  /**
   * A volume fader.
   *
   * Elpy read `offsetX` on click and pointermove, which is cheap and gives the
   * solid-block look, but it breaks the moment the pointer leaves the track and
   * it is invisible to a keyboard. This keeps the look and fixes both: pointer
   * capture means the drag survives leaving the element, and it is a real
   * slider to assistive tech.
   */
  let {
    value,
    accent = 'var(--nuru)',
    disabled = false,
    label,
    onchange,
  }: {
    value: number;
    accent?: string;
    disabled?: boolean;
    label: string;
    onchange: (v: number) => void;
  } = $props();

  let track = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);

  function valueAt(clientX: number): number {
    if (!track) return value;
    const r = track.getBoundingClientRect();
    return Math.min(1, Math.max(0, (clientX - r.left) / r.width));
  }

  function down(e: PointerEvent) {
    if (disabled) return;
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    onchange(valueAt(e.clientX));
  }

  function move(e: PointerEvent) {
    if (!dragging) return;
    onchange(valueAt(e.clientX));
  }

  function up(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }

  function key(e: KeyboardEvent) {
    if (disabled) return;
    // Shift for fine adjustment, matching every other slider on the platform.
    const step = e.shiftKey ? 0.01 : 0.05;
    let next: number | null = null;
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp') next = value + step;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') next = value - step;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = 1;
    if (next === null) return;
    e.preventDefault();
    onchange(Math.min(1, Math.max(0, next)));
  }

  const percent = $derived(Math.round(value * 100));
</script>

<div
  bind:this={track}
  class="fader"
  class:dragging
  class:disabled
  style:--accent={accent}
  style:--fill="{value * 100}%"
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={percent}
  aria-valuetext="{percent}%"
  aria-disabled={disabled}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onkeydown={key}
>
  <div class="rail"></div>
  <div class="fill"></div>
  <div class="thumb"></div>
</div>

<style>
  .fader {
    position: relative;
    height: 22px;
    display: flex;
    align-items: center;
    touch-action: none;
    cursor: default;
  }

  .rail {
    position: absolute;
    inset-inline: 0;
    height: 6px;
    border-radius: var(--r-pill);
    background: rgba(255, 255, 255, 0.07);
    transition: height var(--dur-2) var(--ease-out);
  }

  .fill {
    position: absolute;
    left: 0;
    width: var(--fill);
    height: 6px;
    border-radius: var(--r-pill);
    background: var(--accent);
    /* The glow is what ties a fader to its tile — same colour, same bloom. */
    box-shadow: 0 0 14px -2px var(--accent);
    transition:
      height var(--dur-2) var(--ease-out),
      box-shadow var(--dur-2) var(--ease-out);
  }

  .thumb {
    position: absolute;
    left: var(--fill);
    width: 13px;
    height: 13px;
    margin-left: -6.5px;
    border-radius: 50%;
    background: #fff;
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.5),
      0 0 0 0 var(--accent);
    transform: scale(0.82);
    opacity: 0;
    transition:
      transform var(--dur-3) var(--ease-spring),
      opacity var(--dur-2) var(--ease-out),
      box-shadow var(--dur-3) var(--ease-out);
  }

  /* The thumb only exists when it is wanted. At rest the fader is the solid bar
     from the reference; on approach it becomes a control. */
  .fader:hover .thumb,
  .fader:focus-visible .thumb,
  .fader.dragging .thumb {
    opacity: 1;
    transform: scale(1);
  }

  .fader:hover .rail,
  .fader:hover .fill,
  .fader.dragging .rail,
  .fader.dragging .fill {
    height: 8px;
  }

  .fader.dragging .thumb {
    transform: scale(1.18);
    box-shadow:
      0 2px 6px rgba(0, 0, 0, 0.55),
      0 0 0 6px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .fader.dragging .fill {
    box-shadow: 0 0 22px -1px var(--accent);
  }

  .fader.disabled {
    opacity: 0.35;
  }
  .fader.disabled .thumb {
    display: none;
  }

  /* Keyboard focus shows itself by doing what hover does — the track thickens
     and the thumb appears (see the rules above). No ring. */
  .fader:focus-visible .rail,
  .fader:focus-visible .fill {
    height: 8px;
  }
</style>
