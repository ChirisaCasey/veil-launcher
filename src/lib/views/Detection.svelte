<script lang="ts">
  import { flip } from "svelte/animate";
  import { fade, slide } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import type { InstallSource } from "$lib/api";
  import { detection, settings, toasts } from "$lib/stores";
  import { formatBytes, formatRelative } from "$lib/format";
  import { pickFolder, revealPath } from "$lib/platform";
  import { ms } from "$lib/motion";
  import ViewHeader from "$lib/components/ViewHeader.svelte";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import FolderSearch from "@lucide/svelte/icons/folder-search";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import Pin from "@lucide/svelte/icons/pin";
  import PinOff from "@lucide/svelte/icons/pin-off";
  import ShieldCheck from "@lucide/svelte/icons/shield-check";
  import ShieldAlert from "@lucide/svelte/icons/shield-alert";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import HardDrive from "@lucide/svelte/icons/hard-drive";
  import FileVideo from "@lucide/svelte/icons/file-video";
  import Target from "@lucide/svelte/icons/target";
  import ExternalLink from "@lucide/svelte/icons/external-link";

  const SOURCE: Record<InstallSource, string> = {
    metadata: "Riot metadata",
    registry: "Windows registry",
    scan: "Disk scan",
    override: "Chosen by you",
    none: "Not found",
  };

  const install = $derived(detection.install);
  const result = $derived(detection.result);
  const found = $derived(!!install?.installDir);
  const isOverride = $derived(!!settings.value?.installOverride || install?.source === "override");
  const maxScore = $derived(Math.max(100, ...(result?.candidates.map((c) => c.score) ?? [0])));

  async function changeFolder() {
    try {
      const dir = await pickFolder("Select your VALORANT \"live\" folder", install?.installDir);
      if (dir) await detection.setOverride(dir);
    } catch (e) {
      toasts.error(e, "Couldn't open the folder picker");
    }
  }

  async function reveal(path: string | null | undefined) {
    if (!path) return;
    try {
      await revealPath(path);
    } catch (e) {
      toasts.error(e, "Couldn't open Explorer");
    }
  }
</script>

<ViewHeader title="Detection" subtitle="Where VALORANT lives, and which menu video Veil will replace.">
  {#snippet actions()}
    <button type="button" class="btn" onclick={() => detection.rescan()} disabled={detection.scanning}>
      <RefreshCw size={16} class={detection.scanning ? "spin" : ""} />
      {detection.scanning ? "Scanning…" : "Rescan"}
    </button>
  {/snippet}
</ViewHeader>

<div class="stack">
  <!-- Install -->
  <section class="card card-pad" aria-labelledby="install-title">
    <div class="card-header">
      <div class="head">
        <span class="head-icon"><HardDrive size={18} /></span>
        <div>
          <h2 id="install-title" class="card-title">VALORANT install</h2>
          <p class="card-sub">The <span class="mono">live</span> folder Veil watches.</p>
        </div>
      </div>
      <div class="head-actions">
        {#if isOverride}
          <button type="button" class="btn ghost sm" onclick={() => detection.setOverride(null)} disabled={detection.changingFolder}>
            <RotateCcw size={14} /> Reset to auto
          </button>
        {/if}
        <button type="button" class="btn sm" onclick={changeFolder} disabled={detection.changingFolder}>
          <FolderSearch size={14} /> Change folder
        </button>
      </div>
    </div>

    {#if detection.loadingInstall}
      <div class="skeleton" style:height="54px"></div>
    {:else if install && found}
      <div class="install" in:fade={{ duration: ms(200) }}>
        <div class="path-row">
          <span class="mono path selectable" title={install.installDir}>{install.installDir}</span>
          <button type="button" class="icon-btn" aria-label="Show in Explorer" title="Show in Explorer" onclick={() => reveal(install.installDir)}>
            <ExternalLink size={15} />
          </button>
        </div>
        <div class="chips">
          <span class="chip accent">Source: {SOURCE[install.source]}</span>
          {#if install.writable}
            <span class="chip success"><ShieldCheck size={12} /> Writable</span>
          {:else}
            <span class="chip warning"><ShieldAlert size={12} /> Admin needed</span>
          {/if}
        </div>
        <dl class="details">
          <dt>Menu folder</dt>
          <dd class="mono selectable truncate" title={install.menuDir ?? ""}>{install.menuDir ?? "—"}</dd>
          <dt>Riot Client</dt>
          <dd class="mono selectable truncate" title={install.riotClient ?? ""}>{install.riotClient ?? "Not found"}</dd>
        </dl>
        {#if !install.writable}
          <p class="callout warning" transition:slide={{ duration: ms(180) }}>
            <ShieldAlert size={16} />
            <span>This folder needs administrator rights to change. Windows will ask for permission when Veil swaps the video. That's the only time it needs it.</span>
          </p>
        {/if}
      </div>
    {:else}
      <div class="not-found">
        <p class="callout danger">
          <TriangleAlert size={16} />
          <span>Veil couldn't find VALORANT automatically. Choose the <span class="mono">Riot Games\VALORANT\live</span> folder to continue.</span>
        </p>
        <button type="button" class="btn primary" onclick={changeFolder}><FolderOpen size={16} /> Choose folder</button>
      </div>
    {/if}
  </section>

  <!-- Candidates -->
  <section class="card card-pad" aria-labelledby="cand-title">
    <div class="card-header">
      <div class="head">
        <span class="head-icon"><Target size={18} /></span>
        <div>
          <h2 id="cand-title" class="card-title">Menu videos</h2>
          <p class="card-sub">
            {#if result?.pinned}
              Pinned: Veil always swaps the file you chose.
            {:else}
              Ranked automatically. The top match gets swapped.
            {/if}
          </p>
        </div>
      </div>
      {#if result?.pinned || settings.value?.pinnedTarget}
        <button type="button" class="btn sm soft" onclick={() => detection.pin(null)} disabled={detection.pinning !== null}>
          <PinOff size={14} /> Use auto-detect
        </button>
      {/if}
    </div>

    {#if result?.pinMissing}
      <p class="callout warning pin-missing" transition:slide={{ duration: ms(180) }}>
        <TriangleAlert size={16} />
        <span>
          Your pinned file <strong class="mono">{settings.value?.pinnedTarget}</strong> no longer exists (probably a game
          update). Veil fell back to auto-detect. Pin a new file or switch back to auto.
        </span>
      </p>
    {/if}

    {#if !result && detection.scanning}
      <div class="list">
        {#each Array(3) as _, i (i)}
          <div class="skeleton" style:height="76px" style:border-radius="12px"></div>
        {/each}
      </div>
    {:else if result && result.candidates.length === 0}
      <div class="empty">
        <FileVideo size={22} />
        <p>No menu videos found{result.menuDir ? " in this folder" : ""}. Check the install folder above, then rescan.</p>
      </div>
    {:else if result}
      <ul class="list" class:refreshing={detection.scanning}>
        {#each result.candidates as c, i (c.fileName)}
          {@const isSel = result.selected?.fileName === c.fileName}
          {@const pct = Math.max(3, Math.round((c.score / maxScore) * 100))}
          <li class="cand" class:selected={isSel} animate:flip={{ duration: ms(300), easing: cubicOut }}>
            <div class="rank">{i + 1}</div>
            <div class="cand-main">
              <div class="cand-top">
                <span class="mono name selectable truncate" title={c.path}>{c.fileName}</span>
                {#if isSel}
                  <span class="chip accent">{result.pinned ? "Pinned" : "Will be swapped"}</span>
                {/if}
                {#if c.version}<span class="chip">v{c.version}</span>{/if}
                {#if c.isHomescreen}<span class="chip info">Homescreen</span>{/if}
              </div>
              <div class="score-row">
                <div class="score-bar" class:low={c.score < 40}>
                  <span style:width="{pct}%"></span>
                </div>
                <span class="score-num" aria-label="Score {c.score}">{c.score}</span>
              </div>
              <div class="reason">
                {c.reason}
                <span class="faint">· {formatBytes(c.size)} · modified {formatRelative(c.modified)}</span>
              </div>
            </div>
            <div class="cand-actions">
              {#if isSel && result.pinned}
                <span class="pinned"><Pin size={14} /> Pinned</span>
              {:else}
                <button
                  type="button"
                  class="btn sm"
                  onclick={() => detection.pin(c.fileName)}
                  disabled={detection.pinning !== null}
                  title="Always swap this file"
                >
                  <Pin size={14} /> Pin this file
                </button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .stack {
    display: grid;
    gap: 16px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .head-icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .head-actions {
    display: flex;
    gap: 6px;
    flex: none;
  }
  .install {
    display: grid;
    gap: 12px;
  }
  .path-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 6px 10px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .path {
    flex: 1;
    font-size: 13.5px;
    font-weight: 500;
    overflow-wrap: anywhere;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .details {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 4px 16px;
    margin: 0;
    font-size: 12.5px;
  }
  .details dt {
    color: var(--text-faint);
  }
  .details dd {
    margin: 0;
    color: var(--text-muted);
  }
  .not-found {
    display: grid;
    gap: 12px;
    justify-items: start;
  }

  .pin-missing {
    margin-bottom: 14px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
    transition: opacity 0.2s ease;
  }
  .list.refreshing {
    opacity: 0.6;
  }
  .cand {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
    transition: border-color 0.2s ease, background-color 0.2s ease, box-shadow 0.2s ease;
  }
  .cand.selected {
    background: var(--accent-softer);
    border-color: var(--accent-border);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 25%, transparent) inset;
  }
  .rank {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    font-size: 12px;
    font-weight: 700;
    color: var(--text-muted);
    background: var(--surface-3);
  }
  .selected .rank {
    background: var(--accent-solid);
    color: var(--on-accent);
  }
  .cand-main {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 6px;
  }
  .cand-top {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    font-size: 13px;
    margin-right: 4px;
  }
  .score-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .score-bar {
    flex: 1;
    max-width: 320px;
    height: 6px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .score-bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--accent-solid), var(--accent));
    transform-origin: left center;
    transition: width 0.6s var(--ease-out);
    animation: grow 0.8s var(--ease-out);
  }
  .score-bar.low span {
    background: var(--text-faint);
  }
  @keyframes grow {
    from {
      transform: scaleX(0);
    }
  }
  .score-num {
    font-size: 12px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    min-width: 24px;
  }
  .reason {
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .reason::first-letter {
    text-transform: uppercase;
  }
  .cand-actions {
    flex: none;
  }
  .pinned {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--accent-text);
    padding: 0 8px;
  }
  .empty {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 18px;
    border-radius: var(--radius);
    background: var(--surface-2);
    color: var(--text-muted);
  }
</style>
