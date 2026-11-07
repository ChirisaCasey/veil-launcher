<script lang="ts">
  import { api, type Settings, type SwapTrigger, type ThemeMode } from "$lib/api";
  import { ffmpeg, settings, status, theme, toasts } from "$lib/stores";
  import { ACCENT_PRESETS, FFMPEG_DOWNLOAD_MB } from "$lib/constants";
  import { revealPath } from "$lib/platform";
  import ViewHeader from "$lib/components/ViewHeader.svelte";
  import SettingRow from "$lib/components/SettingRow.svelte";
  import Segmented from "$lib/components/Segmented.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import FfmpegProgress from "$lib/components/FfmpegProgress.svelte";
  import Monitor from "@lucide/svelte/icons/monitor";
  import Sun from "@lucide/svelte/icons/sun";
  import Moon from "@lucide/svelte/icons/moon";
  import Check from "@lucide/svelte/icons/check";
  import Pipette from "@lucide/svelte/icons/pipette";
  import Palette from "@lucide/svelte/icons/palette";
  import Power from "@lucide/svelte/icons/power";
  import Timer from "@lucide/svelte/icons/timer";
  import Clapperboard from "@lucide/svelte/icons/clapperboard";
  import Cpu from "@lucide/svelte/icons/cpu";
  import Download from "@lucide/svelte/icons/download";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import ShieldCheck from "@lucide/svelte/icons/shield-check";
  import LogOut from "@lucide/svelte/icons/log-out";

  const s = $derived(settings.value);
  const app = $derived(settings.app);

  const THEME_OPTIONS: { value: ThemeMode; label: string; icon: typeof Monitor }[] = [
    { value: "system", label: "System", icon: Monitor },
    { value: "light", label: "Light", icon: Sun },
    { value: "dark", label: "Dark", icon: Moon },
  ];

  const TRIGGERS: { value: SwapTrigger; title: string; body: string; badge?: string }[] = [
    {
      value: "riotClientClosed",
      title: "After the Riot Client closes",
      body: "Waits until Riot has finished checking your files and the client window closes. Safest choice.",
      badge: "Recommended",
    },
    {
      value: "gameStarted",
      title: "As soon as VALORANT starts",
      body: "Swaps right when the game process appears. Faster, but if Riot is still checking files it may undo the swap.",
    },
  ];

  const isPreset = $derived(ACCENT_PRESETS.some((p) => p.value.toLowerCase() === theme.accent.toLowerCase()));

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    void settings.update({ [key]: value } as Partial<Settings>);
  }

  async function revealData() {
    if (!app?.dataDir) return;
    try {
      await revealPath(app.dataDir);
    } catch (e) {
      toasts.error(e, "Couldn't open the data folder");
    }
  }

  async function quit() {
    try {
      await api.quitApp();
    } catch (e) {
      toasts.error(e, "Couldn't quit");
    }
  }
</script>

<ViewHeader title="Settings" subtitle="Make Veil look and behave the way you like." />

{#if !s}
  <div class="stack">
    {#each Array(3) as _, i (i)}
      <div class="card card-pad"><div class="skeleton" style:height="120px"></div></div>
    {/each}
  </div>
{:else}
  <div class="stack">
    <!-- Appearance -->
    <section class="card card-pad" aria-labelledby="sec-appearance">
      <h2 class="section-title" id="sec-appearance"><Palette size={17} /> Appearance</h2>
      <SettingRow label="Theme" description="System follows your Windows light/dark setting.">
        <Segmented label="Theme" options={THEME_OPTIONS} value={s.themeMode} onchange={(v) => set("themeMode", v)} />
      </SettingRow>
      <SettingRow label="Accent color" description="Used for the main button, toggles and selection.">
        <div class="swatches" role="radiogroup" aria-label="Accent color">
          {#each ACCENT_PRESETS as p (p.value)}
            {@const active = p.value.toLowerCase() === theme.accent.toLowerCase()}
            <button
              type="button"
              class="swatch"
              class:active
              role="radio"
              aria-checked={active}
              aria-label={p.name}
              title={p.name}
              style:--c={p.value}
              onclick={() => set("accent", p.value)}
            >
              {#if active}<Check size={14} strokeWidth={3} />{/if}
            </button>
          {/each}
          <label class="swatch custom" class:active={!isPreset} title="Custom color" style:--c={theme.accent}>
            <input
              type="color"
              value={theme.accent}
              aria-label="Custom accent color"
              oninput={(e) => settings.updateDebounced({ accent: e.currentTarget.value })}
              onchange={() => settings.flush()}
            />
            <Pipette size={14} />
          </label>
          <span class="hex mono selectable">{theme.accent.toUpperCase()}</span>
        </div>
      </SettingRow>
    </section>

    <!-- Behavior -->
    <section class="card card-pad" aria-labelledby="sec-behavior">
      <h2 class="section-title" id="sec-behavior"><Power size={17} /> Behavior</h2>
      <SettingRow
        label="Run in background"
        description="Keeps watching even when you launch VALORANT from the Riot Client or a shortcut, so your video always applies. Windows may ask for admin permission once to set this up."
      >
        {#snippet extra()}
          {#if s.backgroundMode}
            {#if status.value?.backgroundInstalled}
              <span class="chip success"><ShieldCheck size={12} /> Background task installed</span>
            {:else}
              <span class="chip warning">Not installed yet</span>
            {/if}
          {/if}
        {/snippet}
        <Toggle label="Run in background" checked={s.backgroundMode} onchange={(v) => set("backgroundMode", v)} />
      </SettingRow>
      <SettingRow label="Start with Windows" description="Launch Veil quietly when you sign in.">
        <Toggle label="Start with Windows" checked={s.startWithWindows} onchange={(v) => set("startWithWindows", v)} />
      </SettingRow>
      <SettingRow label="Start minimized" description="Open straight to the system tray instead of showing this window.">
        <Toggle label="Start minimized" checked={s.startMinimized} onchange={(v) => set("startMinimized", v)} />
      </SettingRow>
      <SettingRow label="Notifications" description="Show a desktop notification when your video is applied, restored, or something fails.">
        <Toggle label="Notifications" checked={s.notifications} onchange={(v) => set("notifications", v)} />
      </SettingRow>
    </section>

    <!-- Swap timing -->
    <section class="card card-pad" aria-labelledby="sec-timing">
      <h2 class="section-title" id="sec-timing"><Timer size={17} /> Swap timing</h2>
      <SettingRow label="When to swap" description="Riot checks game files at launch and replaces anything that changed, so the swap has to wait until that's done." stacked>
        <div class="triggers" role="radiogroup" aria-label="When to swap">
          {#each TRIGGERS as t (t.value)}
            {@const active = s.swapTrigger === t.value}
            <button
              type="button"
              class="trigger"
              class:active
              role="radio"
              aria-checked={active}
              onclick={() => set("swapTrigger", t.value)}
            >
              <span class="radio-dot" aria-hidden="true"></span>
              <span class="trigger-text">
                <span class="trigger-title">
                  {t.title}
                  {#if t.badge}<span class="chip accent">{t.badge}</span>{/if}
                </span>
                <span class="trigger-body">{t.body}</span>
              </span>
            </button>
          {/each}
        </div>
      </SettingRow>
      <SettingRow label="Extra delay" description="Wait a little longer after the trigger before swapping. Raise it if the original video sometimes comes back.">
        <div class="slider">
          <input
            type="range"
            min="0"
            max="60"
            step="1"
            value={s.swapDelaySecs}
            aria-label="Extra delay in seconds"
            aria-valuetext="{s.swapDelaySecs} seconds"
            style:--pct="{(s.swapDelaySecs / 60) * 100}%"
            oninput={(e) => settings.updateDebounced({ swapDelaySecs: Number(e.currentTarget.value) })}
            onchange={() => settings.flush()}
          />
          <span class="slider-val">{s.swapDelaySecs} s</span>
        </div>
      </SettingRow>
    </section>

    <!-- Video -->
    <section class="card card-pad" aria-labelledby="sec-video">
      <h2 class="section-title" id="sec-video"><Clapperboard size={17} /> Video</h2>
      <SettingRow label="Keep audio when converting" description="The menu background usually plays silently. Turn this on to keep the original audio track in converted videos.">
        <Toggle label="Keep audio when converting" checked={s.keepAudio} onchange={(v) => set("keepAudio", v)} />
      </SettingRow>
    </section>

    <!-- Advanced -->
    <section class="card card-pad" aria-labelledby="sec-advanced">
      <h2 class="section-title" id="sec-advanced"><Cpu size={17} /> Advanced</h2>
      <SettingRow
        label="ffmpeg"
        description={ffmpeg.installed
          ? "Used to convert videos into a format VALORANT can play."
          : `Optional. Needed only to convert videos that aren't H.264 .mp4. About ${FFMPEG_DOWNLOAD_MB} MB download.`}
      >
        {#snippet extra()}
          {#if ffmpeg.status?.installed}
            <span class="chip success">Installed{ffmpeg.status.version ? ` · ${ffmpeg.status.version}` : ""}</span>
            {#if ffmpeg.status.path}<span class="mono faint selectable path">{ffmpeg.status.path}</span>{/if}
          {:else if ffmpeg.status}
            <span class="chip">Not installed</span>
          {/if}
        {/snippet}
        {#if !ffmpeg.installed}
          <button type="button" class="btn sm" disabled={ffmpeg.installing || !ffmpeg.status} onclick={() => ffmpeg.install()}>
            <Download size={14} /> {ffmpeg.installing ? "Downloading…" : "Install ffmpeg"}
          </button>
        {/if}
      </SettingRow>
      {#if ffmpeg.installing || ffmpeg.error}
        <div class="ff-progress"><FfmpegProgress /></div>
      {/if}
      <SettingRow label="Data folder" description="Your library, backups and settings live here.">
        {#snippet extra()}
          {#if app}<span class="mono faint selectable path">{app.dataDir}</span>{/if}
        {/snippet}
        <button type="button" class="btn sm" disabled={!app} onclick={revealData}><FolderOpen size={14} /> Show in Explorer</button>
      </SettingRow>
      <SettingRow label="Permissions" description="Veil runs as a normal app and only asks for admin rights when the game folder needs them.">
        {#if app}
          {#if app.elevated}
            <span class="chip warning"><ShieldCheck size={12} /> Running as administrator</span>
          {:else}
            <span class="chip">Standard user</span>
          {/if}
        {/if}
      </SettingRow>
      <SettingRow label="Version" description="Veil Launcher">
        <span class="mono muted">{app ? `v${app.version}` : "…"}</span>
      </SettingRow>
      <SettingRow label="Quit Veil" description="Closing the window only hides Veil to the tray. This exits completely and puts the original video back if needed.">
        <button type="button" class="btn sm danger" onclick={quit}><LogOut size={14} /> Quit</button>
      </SettingRow>
    </section>
  </div>
{/if}

<style>
  .stack {
    display: grid;
    gap: 16px;
  }
  .section-title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    margin-bottom: 8px;
  }
  .section-title :global(svg) {
    color: var(--accent-text);
  }

  /* Swatches */
  .swatches {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .swatch {
    position: relative;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--c);
    color: #fff;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.15);
    transition: transform 0.2s var(--ease-spring), box-shadow 0.2s ease;
  }
  .swatch :global(svg) {
    filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.5));
  }
  .swatch:hover {
    transform: scale(1.1);
  }
  .swatch.active {
    box-shadow:
      0 0 0 2px var(--surface),
      0 0 0 4px var(--c),
      0 4px 14px -2px color-mix(in srgb, var(--c) 70%, transparent);
  }
  .swatch:focus-visible {
    border-radius: 50%;
    box-shadow:
      0 0 0 2px var(--surface),
      0 0 0 4px var(--text);
  }
  .swatch.custom {
    cursor: pointer;
    background: conic-gradient(from 0deg, #ff4655, #f59e0b, #10b981, #22d3ee, #7c5cff, #ec4899, #ff4655);
  }
  .swatch.custom.active {
    background: var(--c);
  }
  .swatch.custom:focus-within {
    box-shadow:
      0 0 0 2px var(--surface),
      0 0 0 4px var(--text);
  }
  .swatch.custom input {
    position: absolute;
    inset: 0;
    opacity: 0;
    width: 100%;
    height: 100%;
    cursor: pointer;
    border: none;
    padding: 0;
  }
  .swatch.custom :global(svg) {
    pointer-events: none;
  }
  .hex {
    width: 64px;
    font-size: 12px;
    color: var(--text-muted);
  }

  /* Triggers */
  .triggers {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .trigger {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    text-align: left;
    padding: 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
    transition: border-color 0.2s ease, background-color 0.2s ease, box-shadow 0.2s ease;
  }
  .trigger:hover {
    border-color: var(--border-strong);
  }
  .trigger.active {
    background: var(--accent-softer);
    border-color: var(--accent-border);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 25%, transparent) inset;
  }
  .trigger:focus-visible {
    border-radius: var(--radius);
  }
  .radio-dot {
    flex: none;
    width: 18px;
    height: 18px;
    margin-top: 1px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    transition: border 0.2s var(--ease-spring);
  }
  .active .radio-dot {
    border: 5px solid var(--accent-solid);
    background: var(--on-accent);
  }
  .trigger-text {
    display: grid;
    gap: 4px;
  }
  .trigger-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 13.5px;
  }
  .trigger-body {
    font-size: 12.5px;
    color: var(--text-muted);
  }

  /* Slider */
  .slider {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .slider-val {
    width: 40px;
    text-align: right;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    width: 220px;
    height: 6px;
    border-radius: 999px;
    background: linear-gradient(90deg, var(--accent-solid) 0 var(--pct), var(--track) var(--pct) 100%);
    cursor: pointer;
  }
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    border: 2px solid var(--accent-solid);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
    transition: transform 0.15s var(--ease-spring);
  }
  input[type="range"]:hover::-webkit-slider-thumb {
    transform: scale(1.12);
  }
  input[type="range"]:focus-visible {
    box-shadow: var(--focus-ring);
  }

  .path {
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }
  .ff-progress {
    padding: 0 0 14px;
  }

  @media (max-width: 1000px) {
    .triggers {
      grid-template-columns: 1fr;
    }
  }
</style>
