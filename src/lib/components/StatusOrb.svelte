<script lang="ts">
  import type { Phase } from "$lib/api";
  import { PHASES } from "$lib/format";
  import Check from "@lucide/svelte/icons/check";
  import ShieldCheck from "@lucide/svelte/icons/shield-check";
  import Clock from "@lucide/svelte/icons/clock";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Power from "@lucide/svelte/icons/power";

  let {
    phase,
    size = 56,
    /** Use the bright palette (for placement over the always-dark hero media). */
    media = false,
  }: { phase: Phase; size?: number; media?: boolean } = $props();

  const BRIGHT = {
    accent: "var(--accent)",
    info: "#60a5fa",
    warning: "#fbbf24",
    success: "#34d399",
    danger: "#f87171",
  } as const;

  const ICONS = {
    idle: Power,
    riotClient: ShieldCheck,
    waiting: Clock,
    swapped: Check,
    restoring: RotateCcw,
    error: TriangleAlert,
  };

  const meta = $derived(PHASES[phase]);
  const tone = $derived(media ? BRIGHT[meta.tone] : meta.tone === "accent" ? "var(--accent)" : `var(--${meta.tone})`);
  const Icon = $derived(ICONS[phase]);
</script>

<div class="orb {phase}" style:--size="{size}px" style:--tone={tone} role="img" aria-label={meta.label}>
  <span class="halo"></span>
  <span class="pulse"></span>
  <span class="arc"></span>
  <span class="core">
    {#key phase}
      <span class="icon"><Icon size={Math.round(size * 0.36)} strokeWidth={2.4} /></span>
    {/key}
  </span>
</div>

<style>
  .orb {
    position: relative;
    flex: none;
    width: var(--size);
    height: var(--size);
    display: grid;
    place-items: center;
  }
  .orb > span {
    grid-area: 1 / 1;
  }
  .halo {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    background: radial-gradient(circle, color-mix(in srgb, var(--tone) 45%, transparent) 0%, transparent 70%);
    animation: breathe 3.2s ease-in-out infinite;
  }
  .core {
    position: relative;
    width: 62%;
    height: 62%;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    background: radial-gradient(circle at 35% 30%, color-mix(in srgb, var(--tone), white 25%), var(--tone) 60%, color-mix(in srgb, var(--tone), black 25%));
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--tone), white 20%) inset,
      0 6px 22px -4px color-mix(in srgb, var(--tone) 75%, transparent);
    transition: background 0.4s ease, box-shadow 0.4s ease;
  }
  .core :global(svg) {
    filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.35));
  }
  .icon {
    display: grid;
    place-items: center;
    animation: pop 0.45s var(--ease-spring);
  }
  .arc {
    width: 84%;
    height: 84%;
    border-radius: 50%;
    opacity: 0;
    background: conic-gradient(from 0deg, transparent 0 55%, var(--tone) 100%);
    -webkit-mask: radial-gradient(farthest-side, transparent calc(100% - 3px), #000 calc(100% - 2.5px));
    mask: radial-gradient(farthest-side, transparent calc(100% - 3px), #000 calc(100% - 2.5px));
    transition: opacity 0.3s ease;
  }
  .pulse {
    width: 62%;
    height: 62%;
    border-radius: 50%;
    border: 2px solid var(--tone);
    opacity: 0;
  }

  .riotClient .arc,
  .waiting .arc,
  .restoring .arc {
    opacity: 1;
    animation: spin 1.1s linear infinite;
  }
  .restoring .arc {
    animation-direction: reverse;
  }
  .waiting .arc {
    animation-duration: 1.6s;
  }
  .swapped .pulse {
    animation: ripple 2.4s var(--ease-out) infinite;
  }
  .error .core {
    animation: shake 0.5s ease 1;
  }
  .error .halo {
    animation: none;
    opacity: 0.8;
  }

  @keyframes breathe {
    0%,
    100% {
      transform: scale(0.82);
      opacity: 0.55;
    }
    50% {
      transform: scale(1);
      opacity: 1;
    }
  }
  @keyframes ripple {
    0% {
      transform: scale(1);
      opacity: 0.7;
    }
    100% {
      transform: scale(1.65);
      opacity: 0;
    }
  }
  @keyframes pop {
    0% {
      transform: scale(0.4);
      opacity: 0;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }
  @keyframes shake {
    20%,
    60% {
      transform: translateX(-3px);
    }
    40%,
    80% {
      transform: translateX(3px);
    }
  }
</style>
