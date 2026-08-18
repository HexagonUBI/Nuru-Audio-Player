<script lang="ts">
  let {
    size = 20,
    pulse = false,
    class: className = '',
  }: { size?: number; pulse?: boolean; class?: string } = $props();
</script>

<svg
  class="mark {className}"
  class:pulse
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
>
  <defs>
    <linearGradient id="nuru-core" x1="6" y1="10" x2="18" y2="20" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="var(--nuru-bright, #ffc978)" />
      <stop offset="1" stop-color="var(--nuru, #ffb454)" />
    </linearGradient>
  </defs>

  <circle class="core" cx="12" cy="15.5" r="3.4" fill="url(#nuru-core)" />
  <path
    class="arc arc-1"
    d="M5.6 15.5a6.4 6.4 0 0 1 12.8 0"
    stroke="var(--nuru, #ffb454)"
    stroke-width="1.9"
    stroke-linecap="round"
  />
  <path
    class="arc arc-2"
    d="M1.6 15.5a10.4 10.4 0 0 1 20.8 0"
    stroke="var(--nuru, #ffb454)"
    stroke-width="1.7"
    stroke-linecap="round"
  />
</svg>

<style>
  .mark {
    overflow: visible;
  }

  .arc-1 {
    opacity: 0.72;
  }
  .arc-2 {
    opacity: 0.34;
  }

  .core,
  .arc {
    transform-origin: 12px 15.5px;
    transition:
      opacity var(--dur-4) var(--ease-out),
      transform var(--dur-4) var(--ease-out);
  }

  .pulse .arc-1 {
    animation: breathe 3.4s var(--ease-in-out) infinite;
  }
  .pulse .arc-2 {
    animation: breathe 3.4s var(--ease-in-out) 0.35s infinite;
  }

  @keyframes breathe {
    0%,
    100% {
      opacity: 0.28;
      transform: scale(0.96);
    }
    45% {
      opacity: 0.8;
      transform: scale(1.04);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pulse .arc-1,
    .pulse .arc-2 {
      animation: none;
    }
  }
</style>
