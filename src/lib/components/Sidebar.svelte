<script lang="ts">
  import { Spring } from "svelte/motion";
  import type { LucideIcon } from "@lucide/svelte";
  import House from "@lucide/svelte/icons/house";
  import Film from "@lucide/svelte/icons/film";
  import ScanSearch from "@lucide/svelte/icons/scan-search";
  import Settings from "@lucide/svelte/icons/settings";
  import Info from "@lucide/svelte/icons/info";
  import Logo from "./Logo.svelte";
  import StatusOrb from "./StatusOrb.svelte";
  import { nav, settings, status, videos } from "$lib/stores";
  import type { View } from "$lib/stores/navigation.svelte";
  import { PHASES } from "$lib/format";
  import { prefersReducedMotion } from "$lib/motion";

  const ITEMS: { view: View; label: string; icon: LucideIcon; hint: string }[] = [
    { view: "home", label: "Home", icon: House, hint: "Ctrl+1" },
    { view: "videos", label: "Videos", icon: Film, hint: "Ctrl+2" },
    { view: "detection", label: "Detection", icon: ScanSearch, hint: "Ctrl+3" },
    { view: "settings", label: "Settings", icon: Settings, hint: "Ctrl+4" },
    { view: "about", label: "About", icon: Info, hint: "Ctrl+5" },
  ];

  const ITEM_STEP = 44;
  const activeIndex = $derived(ITEMS.findIndex((i) => i.view === nav.view));
  const indicator = new Spring(0, { stiffness: 0.18, damping: 0.72 });
  $effect(() => {
    void indicator.set(activeIndex * ITEM_STEP, { instant: prefersReducedMotion.current });
  });

  const phase = $derived(status.value?.phase ?? "idle");
  const enabled = $derived(settings.value?.enabled ?? true);
</script>

<aside class="sidebar">
  <div class="brand" data-tauri-drag-region>
    <Logo size={30} />
    <span class="wordmark" data-tauri-drag-region>Veil <span>Launcher</span></span>
  </div>

  <nav aria-label="Main">
    <span class="indicator" style:transform="translateY({indicator.current}px)" aria-hidden="true"></span>
    {#each ITEMS as item (item.view)}
      {@const Icon = item.icon}
      <button
        type="button"
        class="item"
        class:active={nav.view === item.view}
        aria-current={nav.view === item.view ? "page" : undefined}
        title="{item.label} ({item.hint})"
        onclick={() => nav.go(item.view)}
      >
        <Icon size={18} strokeWidth={nav.view === item.view ? 2.25 : 2} />
        <span>{item.label}</span>
        {#if item.view === "videos" && !videos.loading}
          <span class="count">{videos.items.length}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="spacer" data-tauri-drag-region></div>

  <button type="button" class="status" onclick={() => nav.go("home")} aria-label="Status: {PHASES[phase].label}. Open Home">
    <StatusOrb {phase} size={30} />
    <div class="status-text">
      <div class="status-label">{PHASES[phase].label}</div>
      <div class="status-sub truncate">
        {#if !enabled}
          Custom background off
        {:else if videos.selected}
          {videos.selected.name}
        {:else}
          No video selected
        {/if}
      </div>
    </div>
  </button>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-w);
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    padding: 0 12px 12px;
    transition: background-color 0.25s ease;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 72px;
    padding: 12px 8px 4px;
    flex: none;
  }
  .wordmark {
    font-family: var(--font-display);
    font-weight: 750;
    font-size: 16px;
    letter-spacing: -0.02em;
  }
  .wordmark span {
    font-weight: 500;
    color: var(--text-muted);
  }
  nav {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 10px;
  }
  .indicator {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 40px;
    border-radius: 10px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent) 28%, transparent);
    pointer-events: none;
  }
  .indicator::before {
    content: "";
    position: absolute;
    left: -12px;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
    box-shadow: 0 0 10px var(--accent);
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 12px;
    border-radius: 10px;
    color: var(--text-muted);
    font-weight: 550;
    font-size: 14px;
    text-align: left;
    transition: color 0.15s ease, background-color 0.15s ease;
  }
  .item:hover:not(.active) {
    color: var(--text);
    background: color-mix(in srgb, var(--text) 5%, transparent);
  }
  .item.active {
    color: var(--text);
  }
  .item.active :global(svg) {
    color: var(--accent-text);
  }
  .count {
    margin-left: auto;
    min-width: 22px;
    height: 20px;
    padding: 0 6px;
    display: grid;
    place-items: center;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 650;
    color: var(--text-muted);
    background: color-mix(in srgb, var(--text) 9%, transparent);
  }
  .spacer {
    flex: 1;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    text-align: left;
    transition: border-color 0.15s ease, background-color 0.15s ease;
  }
  .status:hover {
    border-color: var(--border-strong);
  }
  .status-text {
    min-width: 0;
    flex: 1;
  }
  .status-label {
    font-size: 13px;
    font-weight: 600;
  }
  .status-sub {
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
