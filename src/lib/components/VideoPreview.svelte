<script lang="ts">
  import type { VideoItem } from "$lib/api";
  import { mediaSrc } from "$lib/platform";
  import { prefersReducedMotion } from "$lib/motion";
  import Film from "@lucide/svelte/icons/film";

  let {
    video,
    /** Play while true (hover previews). */
    active = false,
    /** Loop continuously (hero). Paused under reduced motion unless `active`. */
    autoplay = false,
    iconSize = 28,
  }: { video: VideoItem | null; active?: boolean; autoplay?: boolean; iconSize?: number } = $props();

  const src = $derived(mediaSrc(video?.path));
  let el = $state<HTMLVideoElement>();
  let failed = $state(false);
  let ready = $state(false);

  $effect(() => {
    void src;
    failed = false;
    ready = false;
  });

  const shouldPlay = $derived(autoplay ? !prefersReducedMotion.current || active : active);

  $effect(() => {
    const v = el;
    if (!v || !ready) return;
    if (shouldPlay) {
      v.play().catch(() => {});
    } else {
      v.pause();
      if (!autoplay) {
        try {
          v.currentTime = 0.1;
        } catch {
          /* not seekable yet */
        }
      }
    }
  });

  const showVideo = $derived(!!src && !failed);
</script>

<div class="preview">
  {#if showVideo}
    <video
      bind:this={el}
      src="{src}#t=0.1"
      muted
      loop
      playsinline
      preload={autoplay ? "auto" : "metadata"}
      disablepictureinpicture
      class:ready
      onloadeddata={() => (ready = true)}
      onerror={() => (failed = true)}
      aria-hidden="true"
    ></video>
  {/if}
  {#if !showVideo || !ready}
    <div class="placeholder" aria-hidden="true">
      <Film size={iconSize} strokeWidth={1.5} />
    </div>
  {/if}
</div>

<style>
  .preview {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: var(--media-bg, #0f0f10);
  }
  video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.4s ease;
  }
  video.ready {
    opacity: 1;
  }
  .placeholder {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: rgba(255, 255, 255, 0.28);
  }
</style>
