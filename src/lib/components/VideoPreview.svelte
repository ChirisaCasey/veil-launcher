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

  // Deterministic, pleasant placeholder gradient per video (used in browser preview or on error).
  const hue = $derived.by(() => {
    const s = video?.id ?? "none";
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
    return h;
  });
  const showVideo = $derived(!!src && !failed);
</script>

<div class="preview" class:moving={shouldPlay || (autoplay && !prefersReducedMotion.current)} style:--h={hue}>
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
      <span class="blob a"></span>
      <span class="blob b"></span>
      <span class="grain"></span>
      {#if !video}
        <Film size={iconSize} strokeWidth={1.5} />
      {/if}
    </div>
  {/if}
</div>

<style>
  .preview {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: #0d0d16;
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
    color: rgba(255, 255, 255, 0.35);
    background:
      linear-gradient(160deg, hsl(var(--h) 55% 16%), hsl(calc(var(--h) + 60) 50% 9%));
  }
  .blob {
    position: absolute;
    width: 70%;
    aspect-ratio: 1;
    border-radius: 50%;
    filter: blur(40px);
    opacity: 0.75;
  }
  .blob.a {
    left: -10%;
    top: -25%;
    background: hsl(var(--h) 85% 55% / 0.55);
  }
  .blob.b {
    right: -15%;
    bottom: -35%;
    background: hsl(calc(var(--h) + 70) 85% 55% / 0.45);
  }
  .moving .blob.a {
    animation: drift-a 9s ease-in-out infinite alternate;
  }
  .moving .blob.b {
    animation: drift-b 11s ease-in-out infinite alternate;
  }
  .grain {
    position: absolute;
    inset: 0;
    background: repeating-linear-gradient(0deg, rgba(255, 255, 255, 0.025) 0 1px, transparent 1px 3px);
    mix-blend-mode: overlay;
  }
  @keyframes drift-a {
    to {
      transform: translate(30%, 25%) scale(1.15);
    }
  }
  @keyframes drift-b {
    to {
      transform: translate(-35%, -20%) scale(0.9);
    }
  }
</style>
