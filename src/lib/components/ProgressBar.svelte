<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import { prefersReducedMotion } from "$lib/motion";

  let {
    value,
    label,
    tone = "accent",
    indeterminate = false,
    height = 6,
  }: {
    /** 0–100 */
    value: number;
    label: string;
    tone?: "accent" | "success" | "danger";
    indeterminate?: boolean;
    height?: number;
  } = $props();

  const tween = new Tween(0, { duration: 350, easing: cubicOut });
  $effect(() => {
    const v = Math.max(0, Math.min(100, value));
    void tween.set(v, { duration: prefersReducedMotion.current ? 0 : 350 });
  });
</script>

<div
  class="bar {tone}"
  class:indeterminate
  role="progressbar"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={indeterminate ? undefined : Math.round(value)}
  style:height="{height}px"
>
  <span class="fill" style:width={indeterminate ? "40%" : `${tween.current}%`}></span>
</div>

<style>
  .bar {
    position: relative;
    width: 100%;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: inherit;
    background: var(--accent);
  }
  .success .fill {
    background: var(--success);
    box-shadow: none;
  }
  .danger .fill {
    background: var(--danger);
    box-shadow: none;
  }
  .indeterminate .fill {
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes sheen {
    to {
      transform: translateX(100%);
    }
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(250%);
    }
  }
</style>
