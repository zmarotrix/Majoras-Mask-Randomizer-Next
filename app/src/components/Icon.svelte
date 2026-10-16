<script lang="ts" module>
  // A small built-in outline icon set (24x24, stroke 2). Keeping it local
  // means no icon dependency and nothing to download.
  const PATHS: Record<string, string> = {
    search: "M11 4a7 7 0 1 0 0 14a7 7 0 0 0 0-14z M20 20l-4-4",
    x: "M6 6l12 12 M18 6L6 18",
    check: "M5 12l5 5L20 7",
    "chevron-down": "M6 9l6 6l6-6",
    "chevron-right": "M9 6l6 6l-6 6",
    "arrow-right": "M5 12h14 M13 6l6 6l-6 6",
    "arrow-left": "M19 12H5 M11 6l-6 6l6 6",
    dice: "M6 3h12a3 3 0 0 1 3 3v12a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3V6a3 3 0 0 1 3-3z M7.5 8.5a1 1 0 1 0 2 0a1 1 0 1 0-2 0z M14.5 15.5a1 1 0 1 0 2 0a1 1 0 1 0-2 0z M11 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0z",
    play: "M7 4v16l13-8z",
    cartridge: "M6 3h12v14l-2 2v2H8v-2l-2-2z M9 7h6 M9 10h6",
    sliders: "M4 6h10 M18 6h2 M16 4v4 M4 12h4 M12 12h8 M10 10v4 M4 18h12 M20 18h0 M18 16v4",
    branch: "M6 4v16 M6 8a6 6 0 0 0 6 6h2 M18 14a2 2 0 1 0 0.01 0 M6 20a2 2 0 1 0 0.01 0 M6 4a2 2 0 1 0 0.01 0",
    map: "M3 6l6-3l6 3l6-3v15l-6 3l-6-3l-6 3z M9 3v15 M15 6v15",
    copy: "M8 8h12v12H8z M16 8V4H4v12h4",
    file: "M14 3H6v18h12V7z M14 3v4h4",
    upload: "M12 16V4 M7 9l5-5l5 5 M4 20h16",
    download: "M12 4v12 M7 11l5 5l5-5 M4 20h16",
    grid: "M4 4h6v6H4z M14 4h6v6h-6z M4 14h6v6H4z M14 14h6v6h-6z",
    list: "M9 6h11 M9 12h11 M9 18h11 M5 6h.01 M5 12h.01 M5 18h.01",
    columns: "M4 4h7v16H4z M13 4h7v16h-7z",
    checks: "M4 6h3l1 1 M4 12h3 M4 18h3 M11 6h9 M11 12h9 M11 18h9",
    tags: "M3 12V4h8l9 9l-8 8z M7.5 7.5h.01",
    reset: "M4 4v6h6 M5 10a8 8 0 1 1 2 6",
    alert: "M12 3l9 17H3z M12 10v4 M12 17h.01",
    info: "M12 3a9 9 0 1 0 0 18a9 9 0 0 0 0-18z M12 11v6 M12 7h.01",
    // check types
    chest: "M4 10h16v9H4z M4 10l2-5h12l2 5 M11 13h2v2h-2z",
    user: "M12 4a4 4 0 1 0 0 8a4 4 0 0 0 0-8z M5 20a7 7 0 0 1 14 0",
    shop: "M4 9l2-5h12l2 5 M4 9h16v11H4z M10 20v-6h4v6",
    gamepad: "M6 8h12a4 4 0 0 1 0 8H6a4 4 0 0 1 0-8z M8 11v2 M7 12h2 M15 12h.01 M17 11h.01",
    crown: "M4 18h16 M4 18l-1-10l5 4l4-6l4 6l5-4l-1 10",
    moon: "M20 14A8 8 0 1 1 10 4a6 6 0 0 0 10 10z",
    sun: "M12 8a4 4 0 1 0 0 8a4 4 0 1 0 0-8z M12 2v2 M12 20v2 M4.9 4.9l1.4 1.4 M17.7 17.7l1.4 1.4 M2 12h2 M20 12h2 M4.9 19.1l1.4-1.4 M17.7 6.3l1.4-1.4",
    monitor: "M3 5h18v11H3z M8 20h8 M12 16v4",
    diamond: "M6 4h12l3 5l-9 11L3 9z M3 9h18",
    target: "M12 3a9 9 0 1 0 0 18a9 9 0 0 0 0-18z M12 8a4 4 0 1 0 0 8a4 4 0 0 0 0-8z M12 12h.01",
    eye: "M2 12s4-7 10-7s10 7 10 7s-4 7-10 7S2 12 2 12z M12 9a3 3 0 1 0 0 6a3 3 0 0 0 0-6z",
    sword: "M14 4h6v6L9 21l-6-6z M5 14l5 5 M3 21l3-3",
    sparkles: "M12 3l2 6l6 2l-6 2l-2 6l-2-6l-6-2l6-2z M19 3v4 M17 5h4",
    butterfly: "M12 7v12 M12 9C9 4 3 4 4 9c1 4 5 4 8 3 M12 9c3-5 9-5 8 0c-1 4-5 4-8 3",
    sprout: "M12 20v-8 M12 12c0-4-3-6-7-6c0 4 3 6 7 6 M12 10c0-3 3-5 7-5c0 3-3 5-7 5",
    pot: "M8 4h8 M9 4v3a6 6 0 1 0 6 0V4",
    crate: "M4 4h16v16H4z M4 4l16 16 M20 4L4 20",
    barrel: "M7 3h10c2 3 2 15 0 18H7C5 18 5 6 7 3z M5.5 8h13 M5.5 16h13",
    rock: "M4 18l3-8l5-4l6 3l2 9z",
    grass: "M4 20c2-5 2-9 1-14 M10 20c0-5 1-8 3-12 M16 20c0-4 2-7 4-9 M2 20h20",
    snow: "M12 3v18 M4 7.5l16 9 M4 16.5l16-9",
    hive: "M8 4h8l4 8l-4 8H8l-4-8z M6 9h12 M6 15h12",
    tree: "M12 3l6 9h-4l5 6H5l5-6H6z M12 18v3",
    notebook: "M6 3h12v18H6z M10 7h5 M10 11h5 M3 7h3 M3 12h3 M3 17h3",
    scoop: "M5 8h10v9a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3z M15 10h2a2 2 0 0 1 0 4h-2",
    frog: "M7 5a2 2 0 1 0 0 4 M17 5a2 2 0 1 1 0 4 M4 14c0-4 4-6 8-6s8 2 8 6c0 3-3 5-8 5s-8-2-8-5z",
    flag: "M5 21V4 M5 4h11l-2 4l2 4H5",
    star: "M12 3l2.7 5.6l6.1.9l-4.4 4.3l1 6.1L12 17l-5.4 2.9l1-6.1L3.2 9.5l6.1-.9z",
    bug: "M8 8a4 4 0 0 1 8 0v6a4 4 0 0 1-8 0z M3 10h5 M16 10h5 M3 16h5 M16 16h5 M9 4L7 2 M15 4l2-2",
    help: "M12 3a9 9 0 1 0 0 18a9 9 0 0 0 0-18z M9.5 9a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.7 M12 17h.01",
  };
  export const ICONS = Object.keys(PATHS);
</script>

<script lang="ts">
  let { name, size = 16, label }: { name: string; size?: number; label?: string } = $props();
  const d = $derived(PATHS[name] ?? PATHS.help);
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? "img" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : "true"}
  class="icon"
>
  {#each d.split(" M") as seg, i}
    <path d={i === 0 ? seg : `M${seg}`} />
  {/each}
</svg>

<style>
  .icon {
    flex: none;
    vertical-align: -0.15em;
  }
</style>
