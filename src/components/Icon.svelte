<script lang="ts" module>
  /** 24×24 outline icons, one path each, drawn to a common weight. */
  export const paths = {
    play: 'M8 5.5v13l11-6.5-11-6.5Z',
    pause: 'M9 5.5v13M15 5.5v13',
    timer: 'M12 8.5v4l2.5 2M12 21a8.5 8.5 0 1 0 0-17 8.5 8.5 0 0 0 0 17ZM9.5 2.5h5',
    close: 'M6 6l12 12M18 6L6 18',
    minimize: 'M5 12h14',
    maximize: 'M5.5 5.5h13v13h-13z',
    restore: 'M8 8.5V6.5h9.5V16h-2M5.5 8.5h11V19h-11z',
    settings:
      'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z M19.4 14.5a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-2.7 1.1v.3a2 2 0 1 1-4 0v-.2a1.6 1.6 0 0 0-2.8-1.1l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0-1.1-2.7h-.3a2 2 0 1 1 0-4h.2a1.6 1.6 0 0 0 1.1-2.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 2.7-1.1v-.3a2 2 0 1 1 4 0v.2a1.6 1.6 0 0 0 2.8 1.1l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0 1.1 2.7h.3a2 2 0 1 1 0 4h-.2a1.6 1.6 0 0 0-1.4 1Z',
    save: 'M5.5 4.5h10l3.5 3.5v11a1 1 0 0 1-1 1h-12a1 1 0 0 1-1-1v-13a1 1 0 0 1 1-1ZM8 4.5v5h7v-5M8 19.5v-5.5h8v5.5',
    stack: 'M12 3.5 3 8l9 4.5L21 8l-9-4.5ZM3 12.5 12 17l9-4.5M3 16.5 12 21l9-4.5',
    search: 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14ZM20 20l-4-4',
    trash: 'M4.5 6.5h15M9.5 6.5V4.5h5v2M6.5 6.5l1 13h9l1-13M10.5 10v6M13.5 10v6',
    nook: 'M3.5 10.5 12 4l8.5 6.5V20a.5.5 0 0 1-.5.5H4a.5.5 0 0 1-.5-.5v-9.5ZM9.5 20.5v-6h5v6',
    collapse: 'M9.5 4.5v5h-5M14.5 19.5v-5h5M9.5 9.5 4 15M14.5 14.5 20 9',
    volume: 'M4.5 9.5h3l4-3.5v12l-4-3.5h-3v-5Z',
    check: 'M5 12.5 10 17.5 19 7',
    warn: 'M12 9v4.5M12 17v.5M10.3 4.2 2.6 17.5a1.5 1.5 0 0 0 1.3 2.3h16.2a1.5 1.5 0 0 0 1.3-2.3L13.7 4.2a1.5 1.5 0 0 0-2.6 0Z',
  } as const;

  export type IconName = keyof typeof paths;
</script>

<script lang="ts">
  let {
    name,
    size = 18,
    stroke = 1.7,
    fill = false,
  }: { name: IconName; size?: number; stroke?: number; fill?: boolean } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={fill ? 'currentColor' : 'none'}
  stroke={fill ? 'none' : 'currentColor'}
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name]} />
</svg>
