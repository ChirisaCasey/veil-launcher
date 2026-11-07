<script lang="ts" generics="T extends string">
  import type { LucideIcon } from "@lucide/svelte";

  interface Option {
    value: T;
    label: string;
    icon?: LucideIcon;
  }

  let {
    options,
    value,
    onchange,
    label,
  }: { options: Option[]; value: T; onchange: (v: T) => void; label: string } = $props();

  const index = $derived(Math.max(0, options.findIndex((o) => o.value === value)));

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const dir = e.key === "ArrowRight" ? 1 : -1;
    const next = options[(index + dir + options.length) % options.length];
    onchange(next.value);
    const group = e.currentTarget as HTMLElement;
    queueMicrotask(() => group.querySelector<HTMLElement>('[aria-checked="true"]')?.focus());
  }
</script>

<div
  class="seg"
  role="radiogroup"
  aria-label={label}
  tabindex="-1"
  {onkeydown}
  style:--count={options.length}
  style:--index={index}
>
  <span class="indicator" aria-hidden="true"></span>
  {#each options as opt (opt.value)}
    {@const Icon = opt.icon}
    <button
      type="button"
      role="radio"
      aria-checked={opt.value === value}
      tabindex={opt.value === value ? 0 : -1}
      class:active={opt.value === value}
      onclick={() => onchange(opt.value)}
    >
      {#if Icon}<Icon size={15} />{/if}
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--count), 1fr);
    padding: 3px;
    border-radius: 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    min-width: 280px;
  }
  .indicator {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc((100% - 6px) / var(--count));
    border-radius: 7px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-sm);
    transform: translateX(calc(100% * var(--index)));
    transition: transform 0.28s var(--ease-out);
  }
  :global([data-theme="dark"]) .indicator {
    background: var(--surface-3);
  }
  button {
    position: relative;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    font-size: 13px;
    font-weight: 550;
    color: var(--text-muted);
    border-radius: 7px;
    transition: color 0.15s ease;
  }
  button:hover {
    color: var(--text);
  }
  button.active {
    color: var(--text);
  }
</style>
