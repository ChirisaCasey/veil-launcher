<script lang="ts">
  import type { Phase } from "$lib/api";
  import { PHASES } from "$lib/format";
  import Check from "@lucide/svelte/icons/check";
  import ShieldCheck from "@lucide/svelte/icons/shield-check";
  import Clock from "@lucide/svelte/icons/clock";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Power from "@lucide/svelte/icons/power";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";

  // Flat status badge: tinted square + phase icon (a spinner while busy).
  let { phase, size = 40 }: { phase: Phase; size?: number; media?: boolean } = $props();

  const ICONS = {
    idle: Power,
    riotClient: ShieldCheck,
    waiting: Clock,
    swapped: Check,
    restoring: RotateCcw,
    error: TriangleAlert,
  };

  const meta = $derived(PHASES[phase]);
  // Idle is neutral; only meaningful states get color.
  const tone = $derived(
    phase === "idle" ? "var(--text-muted)" : meta.tone === "accent" ? "var(--accent-text)" : `var(--${meta.tone})`,
  );
  const busy = $derived(phase === "riotClient" || phase === "waiting" || phase === "restoring");
  const Icon = $derived(ICONS[phase]);
</script>

<div
  class="badge"
  class:busy
  style:--size="{size}px"
  style:--tone={tone}
  role="img"
  aria-label={meta.label}
>
  {#if busy}
    <LoaderCircle size={Math.round(size * 0.45)} strokeWidth={2} class="spin" />
  {:else}
    <Icon size={Math.round(size * 0.45)} strokeWidth={2} />
  {/if}
</div>

<style>
  .badge {
    position: relative;
    flex: none;
    width: var(--size);
    height: var(--size);
    display: grid;
    place-items: center;
    border-radius: calc(var(--size) * 0.25);
    color: var(--tone);
    background: color-mix(in srgb, var(--tone) 12%, transparent);
    transition: color 0.2s ease, background-color 0.2s ease;
  }
</style>
