<script lang="ts">
  import { fade } from "svelte/transition";
  import { detection, nav, settings, status, videos } from "$lib/stores";
  import { PHASES, codecLabel, formatDuration, resolutionLabel } from "$lib/format";
  import { ms } from "$lib/motion";
  import type { InstallSource, Phase } from "$lib/api";
  import StatusOrb from "$lib/components/StatusOrb.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import VideoPreview from "$lib/components/VideoPreview.svelte";
  import Rocket from "@lucide/svelte/icons/rocket";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import WandSparkles from "@lucide/svelte/icons/wand-sparkles";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import FileVideo from "@lucide/svelte/icons/file-video";
  import HardDrive from "@lucide/svelte/icons/hard-drive";
  import Archive from "@lucide/svelte/icons/archive";
  import Radio from "@lucide/svelte/icons/radio";
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
    metadata: "Found via Riot metadata",
    registry: "Found via registry",
    scan: "Found by disk scan",
    override: "Folder you chose",
    none: "Set the folder in Detection",
  };

  const launchBlocked = $derived.by(() => {
    if (!s) return "Loading…";
    if (!detection.loadingInstall && !installFound) return "Install not found";
    if (s.gameRunning) return "VALORANT is running";
    if (phase === "riotClient") return "Riot Client is open";
    if (phase === "restoring") return "Restoring original…";
    return null;
  });

  const STEPS: { key: Phase; label: string }[] = [
    { key: "riotClient", label: "Riot file check" },
    { key: "waiting", label: "Game starting" },
    { key: "swapped", label: "Your video in" },
    { key: "restoring", label: "Original restored" },
  ];
  const stepIndex = $derived(STEPS.findIndex((x) => x.key === phase));

  async function toggleEnabled(v: boolean) {
    await settings.update({ enabled: v });
  }
</script>

<div class="home">
  <section class="hero" aria-label="Status">
    <VideoPreview video={enabled ? selected : null} autoplay iconSize={40} />
    <div class="shade" class:off={!enabled}></div>

    <div class="hero-body">
      <div class="top">
        <div class="status-line">
          {#if s}
            <StatusOrb {phase} size={56} media />
          {:else}
            <div class="skeleton orb-skel"></div>
          {/if}
          <div class="status-text" aria-live="polite">
            <div class="eyebrow">Status</div>
            {#key phase}
              <h2 in:fade={{ duration: ms(200) }}>{meta.label}</h2>
            {/key}
            <p>{s?.message ?? meta.description}</p>
          </div>
        </div>

        <div class="master">
          <div class="master-text">
            <span class="master-label">Custom background</span>
            <span class="master-sub">{enabled ? "Applied on launch" : "Off: VALORANT stays stock"}</span>
          </div>
          <Toggle
            size="lg"
            label="Custom background"
            checked={enabled}
            disabled={!settings.value}
            onchange={toggleEnabled}
          />
        </div>
      </div>

      <ol class="stepper" class:dim={stepIndex < 0} aria-label="Launch progress">
        {#each STEPS as step, i (step.key)}
          <li class:done={stepIndex > i} class:current={stepIndex === i}>
            <span class="dot">{#if stepIndex > i}<Check size={10} strokeWidth={3} />{/if}</span>
            <span class="step-label">{step.label}</span>
          </li>
        {/each}
      </ol>

      <div class="bottom">
        <div class="now">
          {#if selected}
            <div class="eyebrow">Selected video</div>
            <div class="now-name truncate">{selected.name}</div>
            <div class="now-meta">
              {#each [resolutionLabel(selected), formatDuration(selected.durationSecs), codecLabel(selected.codec)].filter(Boolean) as m (m)}
                <span class="chip media">{m}</span>
              {/each}
              {#if !selected.compatible}<span class="chip media warning">May need conversion</span>{/if}
            </div>
          {:else if !videos.loading}
            <div class="eyebrow">No video selected</div>
            <button type="button" class="btn sm pick" onclick={() => nav.go("videos")}>
              <Film size={14} /> Choose a video
            </button>
          {/if}
        </div>

        <button
          type="button"
          class="btn primary lg launch"
          disabled={!!launchBlocked || status.launching}
          onclick={() => status.launch()}
        >
          {#if status.launching}
            <LoaderCircle size={20} class="spin" /> Launching…
          {:else if launchBlocked && s}
            {launchBlocked}
          {:else}
            <Rocket size={20} /> Launch VALORANT
          {/if}
        </button>
      </div>
    </div>
  </section>

  <section class="stats" aria-label="Overview">
    <button type="button" class="stat card" onclick={() => nav.go("detection")}>
      <span class="stat-head"><span class="stat-icon"><FileVideo size={18} /></span><ChevronRight size={16} class="stat-go" /></span>
      <span class="stat-label">Target file</span>
      {#if detection.result || s}
        <span class="stat-value mono truncate" title={target ?? ""}>{target ?? "Not found"}</span>
        <span class="stat-sub">{settings.value?.pinnedTarget ? "Pinned by you" : "Auto-detected"}</span>
      {:else}
        <span class="skeleton line"></span>
      {/if}
    </button>

    <button type="button" class="stat card" onclick={() => nav.go("detection")}>
      <span class="stat-head"><span class="stat-icon"><HardDrive size={18} /></span><ChevronRight size={16} class="stat-go" /></span>
      <span class="stat-label">VALORANT install</span>
      {#if install}
        <span class="stat-value" class:bad={!installFound}>{installFound ? "Found" : "Not found"}</span>
        <span class="stat-sub">{SOURCE_LABEL[install.source]}</span>
      {:else}
        <span class="skeleton line"></span>
      {/if}
    </button>

    <div class="stat card">
      <span class="stat-head"><span class="stat-icon"><Archive size={18} /></span></span>
      <span class="stat-label">Original backup</span>
      {#if s}
        <span class="stat-value" class:good={s.hasBackup}>{s.hasBackup ? "Present" : "None yet"}</span>
        <span class="stat-sub">{s.hasBackup ? "The original is safe" : "Made on the first swap"}</span>
      {:else}
        <span class="skeleton line"></span>
      {/if}
    </div>

    <button type="button" class="stat card" onclick={() => nav.go("settings")}>
      <span class="stat-head"><span class="stat-icon"><Radio size={18} /></span><ChevronRight size={16} class="stat-go" /></span>
      <span class="stat-label">Background mode</span>
      {#if settings.value}
        <span class="stat-value" class:good={settings.value.backgroundMode}>{settings.value.backgroundMode ? "On" : "Off"}</span>
        <span class="stat-sub">
          {settings.value.backgroundMode
            ? s?.backgroundInstalled
              ? "Covers every launch"
              : "Setting up…"
            : "Only launches from Veil"}
        </span>
      {:else}
        <span class="skeleton line"></span>
      {/if}
    </button>
  </section>

  <section class="manual card card-pad">
    <div class="manual-text">
      <h3 class="card-title">Manual control</h3>
      <p class="card-sub">Game already running, or something out of sync? Swap your video in or put the original back yourself.</p>
    </div>
    <div class="manual-actions">
      <button
        type="button"
        class="btn"
        disabled={!selected || !target || status.applying}
        title={!selected ? "Select a video first" : !target ? "No target file found" : undefined}
        onclick={() => status.applyNow()}
      >
        {#if status.applying}<LoaderCircle size={16} class="spin" />{:else}<WandSparkles size={16} />{/if}
        Apply now
      </button>
      <button
        type="button"
        class="btn"
        disabled={status.restoring || (!s?.swapped && !s?.hasBackup)}
        onclick={() => status.restoreNow()}
      >
        {#if status.restoring}<LoaderCircle size={16} class="spin" />{:else}<RotateCcw size={16} />{/if}
        Restore original
      </button>
    </div>
  </section>
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* ---------- Hero (always a dark media surface, in both themes) ---------- */
  .hero {
    position: relative;
    min-height: 318px;
    border-radius: var(--radius-xl);
    overflow: hidden;
    isolation: isolate;
    color: #f4f4f8;
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
  }
  .shade {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(90deg, rgba(6, 6, 12, 0.88) 0%, rgba(6, 6, 12, 0.6) 45%, rgba(6, 6, 12, 0.25) 100%),
      linear-gradient(0deg, rgba(6, 6, 12, 0.85) 0%, transparent 55%);
    transition: background-color 0.3s ease;
  }
  .shade.off {
    background-color: rgba(6, 6, 12, 0.35);
  }
  .hero-body {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 24px 26px 24px;
    min-height: 318px;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 20px;
  }
  .status-line {
    display: flex;
    gap: 16px;
    align-items: center;
    min-width: 0;
  }
  .orb-skel {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
  }
  .eyebrow {
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(244, 244, 248, 0.7);
  }
  .status-text h2 {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.02em;
    margin-top: 2px;
  }
  .status-text p {
    margin-top: 4px;
    font-size: 13.5px;
    color: rgba(244, 244, 248, 0.78);
    max-width: 440px;
  }
  .master {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px 10px 16px;
    border-radius: var(--radius);
    background: rgba(10, 10, 18, 0.55);
    border: 1px solid rgba(255, 255, 255, 0.12);
    backdrop-filter: blur(12px);
    flex: none;
  }
  .master-text {
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .master-label {
    font-weight: 650;
    font-size: 13.5px;
  }
  .master-sub {
    font-size: 12px;
    color: rgba(244, 244, 248, 0.7);
  }
  .master :global(.switch:not(.on)) {
    background: rgba(255, 255, 255, 0.16);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .stepper {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    gap: 6px;
    transition: opacity 0.3s ease;
  }
  .stepper.dim {
    opacity: 0.75;
  }
  .stepper li {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    font-size: 12px;
    font-weight: 550;
    color: rgba(244, 244, 248, 0.72);
    transition: background-color 0.3s ease, border-color 0.3s ease, color 0.3s ease;
    min-width: 0;
  }
  .step-label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dot {
    flex: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 1.5px solid rgba(255, 255, 255, 0.35);
    transition: all 0.3s ease;
  }
  li.done {
    color: #f4f4f8;
  }
  li.done .dot {
    background: #34d399;
    border-color: #34d399;
    color: #052e1f;
  }
  li.current {
    color: #fff;
    background: color-mix(in srgb, var(--accent) 26%, rgba(10, 10, 18, 0.5));
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  }
  li.current .dot {
    border-color: var(--accent);
    background: var(--accent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 30%, transparent);
    animation: beat 1.4s ease-in-out infinite;
  }
  @keyframes beat {
    50% {
      box-shadow: 0 0 0 7px color-mix(in srgb, var(--accent) 0%, transparent);
    }
  }

  .bottom {
    margin-top: auto;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
  }
  .now {
    min-width: 0;
    flex: 1;
  }
  .now-name {
    font-size: 17px;
    font-weight: 650;
    margin-top: 2px;
  }
  .now-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .pick {
    margin-top: 8px;
    background: rgba(255, 255, 255, 0.12);
    border-color: rgba(255, 255, 255, 0.2);
    color: #fff;
  }
  .pick:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
  }
  .launch {
    flex: none;
    min-width: 220px;
    height: 52px;
    font-size: 15.5px;
    font-weight: 650;
  }
  .launch:disabled {
    opacity: 1;
    background: rgba(255, 255, 255, 0.12);
    color: rgba(244, 244, 248, 0.8);
    box-shadow: none;
  }

  /* ---------- Stats ---------- */
  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 1px;
    padding: 14px 16px 14px;
    text-align: left;
    border-radius: var(--radius);
    box-shadow: var(--shadow-sm);
    min-width: 0;
  }
  button.stat {
    transition: border-color 0.15s ease, transform 0.2s var(--ease-out), background-color 0.15s ease;
  }
  button.stat:hover {
    border-color: var(--border-strong);
    transform: translateY(-1px);
  }
  .stat-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .stat-icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .stat-label {
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 550;
  }
  .stat-value {
    font-weight: 650;
    font-size: 15px;
  }
  .stat-value.mono {
    font-size: 12.5px;
    font-weight: 600;
  }
  .stat-value.good {
    color: var(--success);
  }
  .stat-value.bad {
    color: var(--danger);
  }
  .stat-sub {
    font-size: 12px;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .stat .line {
    height: 14px;
    width: 80%;
    margin: 4px 0;
  }
  .stat :global(.stat-go) {
    color: var(--text-faint);
    transition: transform 0.2s var(--ease-out);
  }
  .stat:hover :global(.stat-go) {
    transform: translateX(2px);
  }

  /* ---------- Manual ---------- */
  .manual {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    border-radius: var(--radius);
    box-shadow: var(--shadow-sm);
  }
  .manual-text p {
    margin-top: 2px;
  }
  .manual-actions {
    display: flex;
    gap: 8px;
    flex: none;
  }

  @media (max-height: 680px) {
    .hero,
    .hero-body {
      min-height: 280px;
    }
    .hero-body {
      gap: 16px;
      padding: 20px 22px;
    }
  }

  @media (max-width: 1000px) {
    .stepper li {
      padding: 8px;
      gap: 6px;
      font-size: 11.5px;
    }
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
