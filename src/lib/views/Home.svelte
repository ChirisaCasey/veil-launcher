<script lang="ts">
  import { fade } from "svelte/transition";
  import { detection, nav, settings, status, videos } from "$lib/stores";
  import { PHASES, codecLabel, formatDuration, resolutionLabel } from "$lib/format";
  import { ms } from "$lib/motion";
  import type { InstallSource, Phase } from "$lib/api";
  import StatusOrb from "$lib/components/StatusOrb.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import VideoPreview from "$lib/components/VideoPreview.svelte";
  import Play from "@lucide/svelte/icons/play";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import ArrowRightLeft from "@lucide/svelte/icons/arrow-right-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Film from "@lucide/svelte/icons/film";
  import Check from "@lucide/svelte/icons/check";

  const s = $derived(status.value);
  const phase = $derived<Phase>(s?.phase ?? "idle");
  const meta = $derived(PHASES[phase]);
  const enabled = $derived(settings.value?.enabled ?? false);
  const selected = $derived(videos.selected);
  const install = $derived(detection.install);
  const installFound = $derived(!!install?.installDir);
  const target = $derived(s?.target ?? detection.result?.selected?.fileName ?? null);

  const SOURCE_LABEL: Record<InstallSource, string> = {
    metadata: "Riot metadata",
    registry: "Registry",
    scan: "Disk scan",
    override: "Chosen by you",
    none: "Not found",
  };

  const launchBlocked = $derived.by(() => {
    if (!s) return "Loading…";
    if (!detection.loadingInstall && !installFound) return "Install not found";
    if (s.gameRunning) return "VALORANT is running";
    if (phase === "riotClient") return "Riot Client is open";
    if (phase === "restoring") return "Restoring…";
    return null;
  });

  const STEPS: { key: Phase; label: string }[] = [
    { key: "riotClient", label: "File check" },
    { key: "waiting", label: "Game starting" },
    { key: "swapped", label: "Video applied" },
    { key: "restoring", label: "Restored" },
  ];
  const stepIndex = $derived(STEPS.findIndex((x) => x.key === phase));
  const videoMeta = $derived(
    selected
      ? [resolutionLabel(selected), formatDuration(selected.durationSecs), codecLabel(selected.codec)]
          .filter(Boolean)
          .join(" · ")
      : "",
  );
</script>

<div class="home">
  <!-- Status panel: preview frame | status + actions. Nothing is layered over anything else. -->
  <section class="panel card" aria-label="Status">
    <div class="preview-col">
      <div class="frame" class:off={!enabled}>
        {#if selected && enabled}
          <VideoPreview video={selected} autoplay iconSize={28} />
        {:else}
          <div class="empty">
            <Film size={22} strokeWidth={1.75} />
            {#if !enabled}
              <span>Custom background is off</span>
            {:else if !videos.loading}
              <span>No video selected</span>
              <button type="button" class="btn sm" onclick={() => nav.go("videos")}>Choose a video</button>
            {/if}
          </div>
        {/if}
      </div>
      {#if selected}
        <div class="video-info">
          <span class="video-name truncate" title={selected.name}>{selected.name}</span>
          <span class="video-meta">
            {videoMeta}
            {#if !selected.compatible}<span class="warn">· May need conversion</span>{/if}
          </span>
        </div>
      {/if}
    </div>

    <div class="status-col">
      <div class="status-head" aria-live="polite">
        {#if s}
          <StatusOrb {phase} size={36} />
        {:else}
          <div class="skeleton badge-skel"></div>
        {/if}
        <div class="status-text">
          {#key phase}
            <h2 in:fade={{ duration: ms(150) }}>{meta.label}</h2>
          {/key}
          <p>{s?.message ?? meta.description}</p>
        </div>
      </div>

      <ol class="steps" aria-label="Launch progress">
        {#each STEPS as step, i (step.key)}
          <li class:done={stepIndex > i} class:current={stepIndex === i}>
            <span class="dot">{#if stepIndex > i}<Check size={9} strokeWidth={3.5} />{/if}</span>
            <span class="step-label">{step.label}</span>
          </li>
        {/each}
      </ol>

      <div class="actions">
        <button
          type="button"
          class="btn primary lg launch"
          disabled={!!launchBlocked || status.launching}
          onclick={() => status.launch()}
        >
          {#if status.launching}
            <LoaderCircle size={16} class="spin" /> Launching…
          {:else if launchBlocked && s}
            {launchBlocked}
          {:else}
            <Play size={15} fill="currentColor" strokeWidth={0} /> Launch VALORANT
          {/if}
        </button>
        <button
          type="button"
          class="btn"
          disabled={!selected || !target || status.applying}
          title={!selected ? "Select a video first" : !target ? "No target file found" : "Swap your video in now"}
          onclick={() => status.applyNow()}
        >
          {#if status.applying}<LoaderCircle size={15} class="spin" />{:else}<ArrowRightLeft size={15} />{/if}
          Apply now
        </button>
        <button
          type="button"
          class="btn"
          disabled={status.restoring || (!s?.swapped && !s?.hasBackup)}
          title="Put the original video back"
          onclick={() => status.restoreNow()}
        >
          {#if status.restoring}<LoaderCircle size={15} class="spin" />{:else}<RotateCcw size={15} />{/if}
          Restore
        </button>
      </div>
    </div>
  </section>

  <!-- Overview: a plain settings-style list -->
  <section class="list card" aria-label="Overview">
    <div class="row">
      <div class="row-text">
        <span class="row-label">Custom background</span>
        <span class="row-sub">{enabled ? "Applied every time VALORANT launches" : "Off: VALORANT keeps its stock menu"}</span>
      </div>
      <Toggle label="Custom background" checked={enabled} disabled={!settings.value} onchange={(v) => settings.update({ enabled: v })} />
    </div>

    <button type="button" class="row link" onclick={() => nav.go("detection")}>
      <span class="row-label">Target file</span>
      <span class="row-value">
        {#if detection.result || s}
          <span class="mono truncate" title={target ?? ""}>{target ?? "Not found"}</span>
          <span class="faint">{settings.value?.pinnedTarget ? "Pinned" : "Auto"}</span>
        {:else}
          <span class="skeleton line"></span>
        {/if}
      </span>
      <ChevronRight size={16} class="chev" />
    </button>

    <button type="button" class="row link" onclick={() => nav.go("detection")}>
      <span class="row-label">VALORANT install</span>
      <span class="row-value">
        {#if install}
          {#if installFound}
            <span class="mono truncate" title={install.installDir ?? ""}>{install.installDir}</span>
            <span class="faint">{SOURCE_LABEL[install.source]}</span>
          {:else}
            <span class="bad">Not found. Set the folder in Detection</span>
          {/if}
        {:else}
          <span class="skeleton line"></span>
        {/if}
      </span>
      <ChevronRight size={16} class="chev" />
    </button>

    <div class="row">
      <span class="row-label">Original backup</span>
      <span class="row-value">
        {#if s}
          {#if s.hasBackup}<span class="good">Saved</span>{:else}<span class="muted">Created on the first swap</span>{/if}
        {:else}
          <span class="skeleton line"></span>
        {/if}
      </span>
      <span class="chev-space"></span>
    </div>

    <button type="button" class="row link" onclick={() => nav.go("settings")}>
      <span class="row-label">Background mode</span>
      <span class="row-value">
        {#if settings.value}
          {#if settings.value.backgroundMode}
            <span class="good">On</span>
            <span class="faint">{s?.backgroundInstalled ? "Covers every launch" : "Setting up…"}</span>
          {:else}
            <span class="muted">Off</span>
            <span class="faint">Only launches from Veil</span>
          {/if}
        {:else}
          <span class="skeleton line"></span>
        {/if}
      </span>
      <ChevronRight size={16} class="chev" />
    </button>
  </section>
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* ---------- Status panel ---------- */
  .panel {
    display: grid;
    grid-template-columns: minmax(260px, 340px) 1fr;
    gap: 24px;
    padding: 20px;
  }
  .preview-col {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .frame {
    position: relative;
    aspect-ratio: 16 / 9;
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .empty {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-faint);
    font-size: 13px;
  }
  .empty span {
    color: var(--text-muted);
  }
  .empty .btn {
    margin-top: 4px;
  }
  .video-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.35;
  }
  .video-name {
    font-weight: 600;
    font-size: 13.5px;
  }
  .video-meta {
    font-size: 12px;
    color: var(--text-muted);
  }
  .video-meta .warn {
    color: var(--warning);
  }

  .status-col {
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-width: 0;
  }
  .status-head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .badge-skel {
    width: 36px;
    height: 36px;
    flex: none;
  }
  .status-text {
    min-width: 0;
  }
  .status-text h2 {
    font-size: 20px;
    font-weight: 600;
  }
  .status-text p {
    margin-top: 2px;
    font-size: 13px;
    color: var(--text-muted);
    max-width: 460px;
  }

  /* Steps: dots joined by a line, labels underneath. */
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
  .steps li {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--text-faint);
    min-width: 0;
  }
  .steps li:not(:last-child)::after {
    content: "";
    position: absolute;
    top: 6px;
    left: 18px;
    right: 6px;
    height: 1px;
    background: var(--border-strong);
  }
  .steps li.done:not(:last-child)::after {
    background: var(--success);
  }
  .dot {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 1.5px solid var(--track);
    background: var(--surface);
    transition: background-color 0.2s ease, border-color 0.2s ease;
  }
  .step-label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  li.done {
    color: var(--text-muted);
  }
  li.done .dot {
    background: var(--success);
    border-color: var(--success);
    color: var(--surface);
  }
  li.current {
    color: var(--text);
    font-weight: 500;
  }
  li.current .dot {
    border-color: var(--accent);
    background: var(--accent);
  }

  .actions {
    margin-top: auto;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .launch {
    min-width: 172px;
    margin-right: 4px;
  }

  /* ---------- Overview list ---------- */
  .list {
    padding: 0 4px;
  }
  .row {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr) 16px;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-height: 52px;
    padding: 8px 16px;
    text-align: left;
    border-top: 1px solid var(--border);
    font-size: 13px;
  }
  .row:first-child {
    border-top: none;
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .row.link {
    border-radius: var(--radius-sm);
    transition: background-color 0.12s ease;
  }
  .row.link:hover {
    background: color-mix(in srgb, var(--text) 4%, transparent);
  }
  .row-text {
    display: flex;
    flex-direction: column;
    line-height: 1.35;
  }
  .row-label {
    font-weight: 500;
    color: var(--text);
  }
  .row-sub {
    font-size: 12px;
    color: var(--text-muted);
  }
  .row-value {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
    color: var(--text);
  }
  .row-value .mono {
    font-size: 12.5px;
  }
  .row-value .faint {
    flex: none;
    font-size: 12px;
  }
  .row .line {
    height: 12px;
    width: 50%;
  }
  .good {
    color: var(--success);
  }
  .bad {
    color: var(--danger);
  }
  .row :global(.chev) {
    color: var(--text-faint);
  }

  @media (max-width: 1000px) {
    .panel {
      grid-template-columns: minmax(220px, 280px) 1fr;
      gap: 20px;
    }
    .row {
      grid-template-columns: 140px minmax(0, 1fr) 16px;
    }
  }
</style>
