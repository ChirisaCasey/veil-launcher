<script lang="ts">
  import { Spring } from "svelte/motion";
  import type { LucideIcon } from "@lucide/svelte";
  import House from "@lucide/svelte/icons/house";
  import Film from "@lucide/svelte/icons/film";
  import ScanSearch from "@lucide/svelte/icons/scan-search";
  import Settings from "@lucide/svelte/icons/settings";
  import SquareTerminal from "@lucide/svelte/icons/square-terminal";
  import Logo from "./Logo.svelte";
  import StatusOrb from "./StatusOrb.svelte";
  import { nav, settings, status, toasts, videos } from "$lib/stores";
  import type { View } from "$lib/stores/navigation.svelte";
  import { PHASES } from "$lib/format";
  import { prefersReducedMotion } from "$lib/motion";
  import { REPO_URL, VAL_PATCHER_URL } from "$lib/constants";
  import { openExternal } from "$lib/platform";

  const ITEMS: { view: View; label: string; icon: LucideIcon; hint: string }[] = [
    { view: "home", label: "Home", icon: House, hint: "Ctrl+1" },
    { view: "videos", label: "Videos", icon: Film, hint: "Ctrl+2" },
    { view: "detection", label: "Detection", icon: ScanSearch, hint: "Ctrl+3" },
    { view: "settings", label: "Settings", icon: Settings, hint: "Ctrl+4" },
  ];

  async function open(url: string) {
    try {
      await openExternal(url);
    } catch (e) {
      toasts.error(e, "Couldn't open link");
    }
  }

  const ITEM_STEP = 38; // item height 36 + gap 2
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
    <Logo size={22} />
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
    <StatusOrb {phase} size={28} />
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

  <div class="links">
    <button type="button" class="link" aria-label="Veil Launcher on GitHub" onclick={() => open(REPO_URL)}>
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" fill="currentColor">
        <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
      </svg>
      <span class="tip" role="tooltip">
        <strong>Veil Launcher on GitHub</strong>
        <span>Source code, releases and issues</span>
      </span>
    </button>
    <button type="button" class="link" aria-label="Val-Patcher, the lite .bat version" onclick={() => open(VAL_PATCHER_URL)}>
      <SquareTerminal size={16} />
      <span class="tip" role="tooltip">
        <strong>Val-Patcher</strong>
        <span>The lite .bat version: a single script that shows exactly what the swap does</span>
      </span>
    </button>
    <span class="version" title="Not affiliated with Riot Games">v{settings.app?.version ?? "…"}</span>
  </div>
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
    height: 56px;
    padding: 8px 12px 0;
    flex: none;
  }
  .wordmark {
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 14.5px;
  }
  .wordmark span {
    font-weight: 400;
    color: var(--text-muted);
  }
  nav {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 10px;
  }
  .indicator {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 36px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--text) 7%, transparent);
    pointer-events: none;
  }
  .indicator::before {
    content: "";
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 36px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-weight: 500;
    font-size: 13.5px;
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

  /* Logo follows the accent color. */
  .brand {
    --logo-color: var(--accent);
  }

  /* Footer links: small icons with a detail card on hover/focus. */
  .links {
    position: relative;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 2px;
    margin-top: 8px;
    padding: 0 2px;
  }
  .link {
    position: relative;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-sm);
    color: var(--text-faint);
    transition: color 0.12s ease, background-color 0.12s ease;
  }
  .link:hover,
  .link:focus-visible {
    color: var(--text);
    background: color-mix(in srgb, var(--text) 7%, transparent);
  }
  .tip {
    position: absolute;
    left: 0;
    bottom: calc(100% + 8px);
    width: 230px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 9px 11px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-lg);
    text-align: left;
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-muted);
    opacity: 0;
    transform: translateY(4px);
    pointer-events: none;
    transition: opacity 0.14s ease, transform 0.14s ease;
    transition-delay: 0s;
  }
  .tip strong {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text);
  }
  .link:hover .tip,
  .link:focus-visible .tip {
    opacity: 1;
    transform: none;
    transition-delay: 0.25s;
  }
  .version {
    margin-left: auto;
    padding-right: 6px;
    font-size: 11.5px;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }
</style>
