<script lang="ts">
  import type { VideoItem } from "$lib/api";
  import { codecLabel, formatBytes, formatDuration, formatRelative, resolutionLabel } from "$lib/format";
  import VideoPreview from "./VideoPreview.svelte";
  import Check from "@lucide/svelte/icons/check";
  import Trash2 from "@lucide/svelte/icons/trash-2";
  import WandSparkles from "@lucide/svelte/icons/wand-sparkles";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

  let {
    video,
    selected,
    onselect,
    onremove,
    onconvert,
  }: {
    video: VideoItem;
    selected: boolean;
    onselect: () => void;
    onremove: () => void;
    onconvert: () => void;
  } = $props();

  let hover = $state(false);
  const badges = $derived(
    [resolutionLabel(video), formatDuration(video.durationSecs), codecLabel(video.codec)].filter(
      (b): b is string => !!b,
    ),
  );
</script>

<article
  class="vcard"
  class:selected
  onmouseenter={() => (hover = true)}
  onmouseleave={() => (hover = false)}
  onfocusin={() => (hover = true)}
  onfocusout={() => (hover = false)}
>
  <button
    type="button"
    class="thumb"
    role="radio"
    aria-checked={selected}
    aria-label="Use {video.name} as the background"
    onclick={onselect}
  >
    <VideoPreview {video} active={hover} />
    <span class="thumb-shade"></span>
    {#if !video.compatible}
      <span class="chip media warning flag" title="The game may not play this codec. Convert it to H.264 to be safe.">
        <TriangleAlert size={12} /> May need conversion
      </span>
    {/if}
    <span class="radio" aria-hidden="true">
      {#if selected}<Check size={13} strokeWidth={3} />{/if}
    </span>
    <span class="badges">
      {#each badges as b (b)}
        <span class="chip media">{b}</span>
      {/each}
    </span>
  </button>

  <div class="info">
    <div class="text">
      <div class="name truncate" title={video.name}>{video.name}</div>
      <div class="sub">{formatBytes(video.size)} · added {formatRelative(video.addedAt)}</div>
    </div>
    <div class="actions">
      {#if !video.compatible}
        <button type="button" class="btn sm soft" onclick={onconvert} title="Convert to H.264 with ffmpeg">
          <WandSparkles size={14} /> Convert
        </button>
      {/if}
      <button type="button" class="icon-btn danger" aria-label="Remove {video.name}" title="Remove" onclick={onremove}>
        <Trash2 size={16} />
      </button>
    </div>
  </div>
</article>

<style>
  .vcard {
    position: relative;
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-sm);
    transition:
      transform 0.25s var(--ease-out),
      box-shadow 0.25s ease,
      border-color 0.2s ease;
  }
  .vcard:hover {
    border-color: var(--border-strong);
  }
  .vcard.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .thumb {
    position: relative;
    display: block;
    width: 100%;
    aspect-ratio: 16 / 9;
    border-radius: calc(var(--radius-lg) - 1px) calc(var(--radius-lg) - 1px) 0 0;
    overflow: hidden;
    isolation: isolate;
  }
  .thumb:focus-visible {
    box-shadow: inset 0 0 0 3px var(--accent);
    border-radius: calc(var(--radius-lg) - 1px) calc(var(--radius-lg) - 1px) 0 0;
  }
  .thumb-shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(0deg, rgba(5, 5, 10, 0.65) 0%, transparent 45%);
    pointer-events: none;
  }
  .badges {
    position: absolute;
    left: 10px;
    bottom: 10px;
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  .flag {
    position: absolute;
    left: 10px;
    top: 10px;
  }
  .radio {
    position: absolute;
    right: 10px;
    top: 10px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 2px solid rgba(255, 255, 255, 0.75);
    background: rgba(0, 0, 0, 0.45);
    color: var(--on-accent);
    opacity: 0;
    transform: scale(0.85);
    transition: all 0.2s var(--ease-spring);
  }
  .vcard:hover .radio,
  .thumb:focus-visible .radio,
  .selected .radio {
    opacity: 1;
    transform: scale(1);
  }
  .selected .radio {
    background: var(--accent-solid);
    border-color: var(--accent-solid);
  }
  .info {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 10px 10px 14px;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    font-size: 13.5px;
  }
  .sub {
    font-size: 12px;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }
</style>
