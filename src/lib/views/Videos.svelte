<script lang="ts">
  import { onMount } from "svelte";
  import { fade, scale, slide } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { cubicOut } from "svelte/easing";
  import type { VideoItem } from "$lib/api";
  import { ffmpeg, settings, toasts, videos } from "$lib/stores";
  import { basename, extension } from "$lib/format";
  import { pickVideos, watchFileDrops } from "$lib/platform";
  import { FFMPEG_DOWNLOAD_MB, VIDEO_EXTENSIONS } from "$lib/constants";
  import { ms } from "$lib/motion";
  import ViewHeader from "$lib/components/ViewHeader.svelte";
  import VideoCard from "$lib/components/VideoCard.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import ProgressBar from "$lib/components/ProgressBar.svelte";
  import FfmpegProgress from "$lib/components/FfmpegProgress.svelte";
  import Plus from "@lucide/svelte/icons/plus";
  import Upload from "@lucide/svelte/icons/upload";
  import Film from "@lucide/svelte/icons/film";
  import WandSparkles from "@lucide/svelte/icons/wand-sparkles";
  import Download from "@lucide/svelte/icons/download";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import X from "@lucide/svelte/icons/x";
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";

  interface ConvertItem {
    path: string;
    name: string;
    reason: "format" | "codec";
  }

  let root = $state<HTMLElement>();
  let dragging = $state(false);
  let picking = $state(false);
  let convertPrompt = $state<ConvertItem[] | null>(null);
  let ffmpegPrompt = $state<ConvertItem[] | null>(null);
  let removeTarget = $state<VideoItem | null>(null);

  const selectedId = $derived(settings.value?.selectedVideoId ?? null);
  const count = $derived(videos.items.length);
  const empty = $derived(!videos.loading && count === 0 && videos.importing.length === 0);

  onMount(() => {
    if (!root) return;
    return watchFileDrops(root, {
      onEnter: () => (dragging = true),
      onLeave: () => (dragging = false),
      onDrop: (paths) => {
        dragging = false;
        void addPaths(paths);
      },
    });
  });

  async function pick() {
    if (picking) return;
    picking = true;
    try {
      const paths = await pickVideos();
      if (paths.length) await addPaths(paths);
    } catch (e) {
      toasts.error(e, "Couldn't open the file picker");
    } finally {
      picking = false;
    }
  }

  async function addPaths(paths: string[]) {
    const toConvert: ConvertItem[] = [];
    const imports: Promise<void>[] = [];
    for (const p of paths) {
      const ext = extension(p);
      const name = basename(p);
      if (!VIDEO_EXTENSIONS.includes(ext)) {
        toasts.warning(`${name} isn't a supported video file.`, "Skipped");
        continue;
      }
      if (ext !== "mp4") {
        toConvert.push({ path: p, name, reason: "format" });
        continue;
      }
      imports.push(
        videos.importFile(p).then((v) => {
          if (!v) return;
          if (!v.compatible) {
            toConvert.push({ path: v.path, name: v.name, reason: "codec" });
          } else {
            toasts.success(`${v.name} was added to your library.`, "Video added");
            if (!settings.value?.selectedVideoId) void videos.select(v.id);
          }
        }),
      );
    }
    await Promise.all(imports);
    if (toConvert.length) convertPrompt = toConvert;
  }

  function requestConvert(items: ConvertItem[]) {
    convertPrompt = null;
    if (!ffmpeg.installed) {
      ffmpegPrompt = items;
      return;
    }
    for (const it of items) void videos.transcode(it.path, it.name);
  }

  async function installAndConvert() {
    const items = ffmpegPrompt ?? [];
    const ok = await ffmpeg.install();
    if (!ok) return;
    ffmpegPrompt = null;
    for (const it of items) void videos.transcode(it.path, it.name);
  }

  async function confirmRemove() {
    const v = removeTarget;
    removeTarget = null;
    if (v) await videos.remove(v);
  }
</script>

<div class="videos" bind:this={root}>
  <ViewHeader
    title="Videos"
    subtitle={videos.loading ? "Loading your library…" : `${count} video${count === 1 ? "" : "s"} in your library. Drop files here to add more.`}
  >
    {#snippet actions()}
      <button type="button" class="btn primary" onclick={pick} disabled={picking}>
        <Plus size={17} /> Add video
      </button>
    {/snippet}
  </ViewHeader>

  {#if videos.jobs.length}
    <section class="jobs" aria-label="Conversions" transition:slide={{ duration: ms(220) }}>
      {#each videos.jobs as job (job.id)}
        <div class="job card" animate:flip={{ duration: ms(220) }} transition:slide={{ duration: ms(200) }}>
          <span class="job-icon" class:done={job.done && !job.error} class:err={!!job.error}>
            {#if job.error}<CircleAlert size={17} />{:else if job.done}<CircleCheck size={17} />{:else}<WandSparkles size={17} />{/if}
          </span>
          <div class="job-main">
            <div class="job-row">
              <span class="job-name truncate">{job.name}</span>
              <span class="job-stage">{job.error ? "Failed" : job.stage}</span>
              <span class="job-pct">{Math.round(job.percent)}%</span>
            </div>
            <ProgressBar
              value={job.percent}
              label="Converting {job.name}"
              tone={job.error ? "danger" : job.done ? "success" : "accent"}
            />
            {#if job.error}<p class="job-err selectable">{job.error}</p>{/if}
          </div>
          {#if job.error || job.done}
            <button type="button" class="icon-btn" aria-label="Dismiss" onclick={() => videos.dismissJob(job.id)}><X size={15} /></button>
          {/if}
        </div>
      {/each}
    </section>
  {/if}

  {#if empty}
    <div class="empty" in:fade={{ duration: ms(200) }}>
      <div class="empty-art">
        <span class="ring r1"></span>
        <span class="ring r2"></span>
        <span class="empty-icon"><Film size={30} /></span>
      </div>
      <h2>Your library is empty</h2>
      <p>Add an .mp4 to use as your VALORANT menu background. Other formats can be converted with ffmpeg.</p>
      <button type="button" class="btn primary lg" onclick={pick}><Plus size={18} /> Add your first video</button>
      <span class="hint">or drag and drop video files anywhere in this view</span>
    </div>
  {:else}
    <div class="grid" role="radiogroup" aria-label="Background video">
      {#if videos.loading}
        {#each Array(6) as _, i (i)}
          <div class="skel-card">
            <div class="skeleton skel-thumb"></div>
            <div class="skel-info">
              <div class="skeleton skel-line" style:width="70%"></div>
              <div class="skeleton skel-line short"></div>
            </div>
          </div>
        {/each}
      {:else}
        {#each videos.importing as name, i (name + i)}
          <div class="skel-card" in:scale={{ start: 0.95, duration: ms(200) }}>
            <div class="skeleton skel-thumb"><span class="importing">Adding {name}…</span></div>
            <div class="skel-info">
              <div class="skeleton skel-line" style:width="70%"></div>
              <div class="skeleton skel-line short"></div>
            </div>
          </div>
        {/each}
        {#each videos.items as v (v.id)}
          <div
            class="cell"
            animate:flip={{ duration: ms(300), easing: cubicOut }}
            in:scale={{ start: 0.94, duration: ms(260), easing: cubicOut }}
            out:scale={{ start: 0.94, duration: ms(180) }}
          >
            <VideoCard
              video={v}
              selected={v.id === selectedId}
              onselect={() => videos.select(v.id)}
              onremove={() => (removeTarget = v)}
              onconvert={() => requestConvert([{ path: v.path, name: v.name, reason: "codec" }])}
            />
          </div>
        {/each}
      {/if}
    </div>
  {/if}

  {#if dragging}
    <div class="dropzone" transition:fade={{ duration: ms(140) }}>
      <div class="drop-inner">
        <span class="drop-icon"><Upload size={30} /></span>
        <strong>Drop to add to your library</strong>
        <span>.mp4 is added directly; other formats can be converted</span>
      </div>
    </div>
  {/if}
</div>

<!-- Convert prompt -->
<Modal open={!!convertPrompt} title="Convert with ffmpeg?" onclose={() => (convertPrompt = null)} width={480}>
  {#snippet icon()}<WandSparkles size={19} />{/snippet}
  {#if convertPrompt}
    {@const allFormat = convertPrompt.every((c) => c.reason === "format")}
    <p>
      {#if allFormat}
        VALORANT only plays .mp4 videos. Veil can convert
        {convertPrompt.length === 1 ? "this file" : "these files"} into an H.264 .mp4 copy in your library.
      {:else}
        {convertPrompt.length === 1 ? "This video uses" : "Some of these use"} a format the game may not play. Converting
        makes an H.264 .mp4 copy that's known to work.
      {/if}
    </p>
    <ul class="file-list">
      {#each convertPrompt as it (it.path)}
        <li>
          <Film size={15} />
          <span class="truncate">{it.name}</span>
          <span class="chip {it.reason === 'format' ? 'info' : 'warning'}">{it.reason === "format" ? "Not .mp4" : "Codec"}</span>
        </li>
      {/each}
    </ul>
    <p class="faint small">Your original file is never modified. Audio is {settings.value?.keepAudio ? "kept" : "removed"} (change this in Settings).</p>
  {/if}
  {#snippet actions()}
    <button type="button" class="btn ghost" onclick={() => (convertPrompt = null)}>
      {convertPrompt?.every((c) => c.reason === "codec") ? "Keep as is" : "Not now"}
    </button>
    <button type="button" class="btn primary" data-autofocus onclick={() => convertPrompt && requestConvert(convertPrompt)}>
      <WandSparkles size={16} /> Convert with ffmpeg
    </button>
  {/snippet}
</Modal>

<!-- ffmpeg install -->
<Modal
  open={!!ffmpegPrompt}
  title="Get ffmpeg to convert videos"
  onclose={() => (ffmpegPrompt = null)}
  dismissible={!ffmpeg.installing}
  width={480}
>
  {#snippet icon()}<Download size={19} />{/snippet}
  <p>
    Converting uses <strong>ffmpeg</strong>, a free open-source video tool. It's an optional one-time download of about
    {FFMPEG_DOWNLOAD_MB} MB, stored in Veil's data folder. Nothing else on your PC changes.
  </p>
  <FfmpegProgress />
  {#snippet actions()}
    <button type="button" class="btn ghost" disabled={ffmpeg.installing} onclick={() => (ffmpegPrompt = null)}>Cancel</button>
    <button type="button" class="btn primary" data-autofocus disabled={ffmpeg.installing} onclick={installAndConvert}>
      {#if ffmpeg.installing}
        <LoaderCircle size={16} class="spin" /> Downloading…
      {:else}
        <Download size={16} /> {ffmpeg.error ? "Try again" : `Download & convert`}
      {/if}
    </button>
  {/snippet}
</Modal>

<!-- Remove confirm -->
<Modal open={!!removeTarget} title="Remove this video?" onclose={() => (removeTarget = null)} width={420}>
  {#snippet icon()}<Trash2 size={19} />{/snippet}
  <p>
    <strong class="selectable">{removeTarget?.name}</strong> will be deleted from Veil's library folder.
    {#if removeTarget && removeTarget.id === selectedId}
      It's your current background, so no custom video will be applied until you pick another.
    {/if}
  </p>
  {#snippet actions()}
    <button type="button" class="btn ghost" data-autofocus onclick={() => (removeTarget = null)}>Cancel</button>
    <button type="button" class="btn danger-solid" onclick={confirmRemove}><Trash2 size={16} /> Remove</button>
  {/snippet}
</Modal>

<style>
  .videos {
    position: relative;
    min-height: 100%;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(232px, 1fr));
    gap: 16px;
  }
  .cell {
    min-width: 0;
  }

  /* Skeletons */
  .skel-card {
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    overflow: hidden;
  }
  .skel-thumb {
    aspect-ratio: 16 / 9;
    border-radius: 0;
    display: grid;
    place-items: center;
  }
  .importing {
    position: relative;
    z-index: 1;
    font-size: 12px;
    color: var(--text-muted);
    padding: 0 12px;
    text-align: center;
  }
  .skel-info {
    padding: 12px 14px;
    display: grid;
    gap: 8px;
  }
  .skel-line {
    height: 11px;
  }
  .skel-line.short {
    width: 45%;
  }

  /* Jobs */
  .jobs {
    display: grid;
    gap: 10px;
    margin-bottom: 18px;
  }
  .job {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    border-radius: var(--radius);
    box-shadow: var(--shadow-sm);
  }
  .job-icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .job-icon.done {
    background: color-mix(in srgb, var(--success) 15%, transparent);
    color: var(--success);
  }
  .job-icon.err {
    background: color-mix(in srgb, var(--danger) 15%, transparent);
    color: var(--danger);
  }
  .job-main {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 8px;
  }
  .job-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    font-size: 13px;
  }
  .job-name {
    font-weight: 600;
    flex: 1;
  }
  .job-stage {
    color: var(--text-muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .job-pct {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    width: 38px;
    text-align: right;
  }
  .job-err {
    font-size: 12px;
    color: var(--danger);
  }

  /* Empty */
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 56px 24px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
  }
  .empty h2 {
    font-size: 19px;
    margin-top: 18px;
  }
  .empty p {
    margin: 6px 0 20px;
    color: var(--text-muted);
    max-width: 420px;
  }
  .hint {
    margin-top: 12px;
    font-size: 12.5px;
    color: var(--text-faint);
  }
  .empty-art {
    position: relative;
    width: 96px;
    height: 96px;
    display: grid;
    place-items: center;
  }
  .empty-art > * {
    grid-area: 1 / 1;
  }
  .empty-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-lg);
    display: grid;
    place-items: center;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }
  .ring {
    display: none;
  }

  /* Drop overlay */
  .dropzone {
    position: fixed;
    inset: var(--titlebar-h) 0 0 var(--sidebar-w);
    z-index: 40;
    padding: 20px;
    background: color-mix(in srgb, var(--bg) 85%, transparent);
    pointer-events: none;
  }
  .drop-inner {
    height: 100%;
    border-radius: var(--radius-xl);
    border: 2px dashed var(--accent);
    background: var(--accent-softer);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 13px;
  }
  .drop-inner strong {
    color: var(--text);
    font-size: 17px;
  }
  .drop-icon {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    margin-bottom: 8px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  @keyframes bob {
    50% {
      transform: translateY(-6px);
    }
  }

  /* Modal content */
  .file-list {
    list-style: none;
    margin: 0;
    padding: 6px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
    display: grid;
    gap: 2px;
    max-height: 180px;
    overflow: auto;
  }
  .file-list li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    color: var(--text);
    font-size: 13px;
  }
  .file-list li :global(svg) {
    flex: none;
    color: var(--text-faint);
  }
  .file-list li .truncate {
    flex: 1;
  }
  .small {
    font-size: 12.5px;
  }
  strong {
    color: var(--text);
  }
</style>
