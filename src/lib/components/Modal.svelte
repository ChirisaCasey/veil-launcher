<script lang="ts">
  import type { Snippet } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { ms } from "$lib/motion";
  import X from "@lucide/svelte/icons/x";

  let {
    open,
    title,
    onclose,
    dismissible = true,
    width = 460,
    icon,
    children,
    actions,
  }: {
    open: boolean;
    title: string;
    onclose: () => void;
    dismissible?: boolean;
    width?: number;
    icon?: Snippet;
    children: Snippet;
    actions?: Snippet;
  } = $props();

  let panel = $state<HTMLElement>();
  let restoreFocus: HTMLElement | null = null;
  const titleId = `modal-${Math.random().toString(36).slice(2, 8)}`;

  $effect(() => {
    if (!open || !panel) return;
    restoreFocus = document.activeElement as HTMLElement | null;
    const target =
      panel.querySelector<HTMLElement>("[data-autofocus]") ??
      panel.querySelector<HTMLElement>(".actions button:last-child") ??
      panel;
    target.focus();
    return () => restoreFocus?.focus?.();
  });

  function focusables(): HTMLElement[] {
    if (!panel) return [];
    return Array.from(
      panel.querySelectorAll<HTMLElement>('button:not(:disabled), [href], input:not(:disabled), [tabindex]:not([tabindex="-1"])'),
    );
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && dismissible) {
      e.stopPropagation();
      onclose();
      return;
    }
    if (e.key === "Tab") {
      const els = focusables();
      if (els.length === 0) return;
      const first = els[0];
      const last = els[els.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }
</script>

{#if open}
  <div class="backdrop" transition:fade={{ duration: ms(160) }}>
    <button
      type="button"
      class="scrim"
      tabindex="-1"
      aria-label="Close dialog"
      onclick={() => dismissible && onclose()}
    ></button>
    <div
      bind:this={panel}
      class="panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabindex="-1"
      style:width="min({width}px, calc(100vw - 48px))"
      {onkeydown}
      transition:scale={{ start: 0.96, duration: ms(200), easing: cubicOut }}
    >
      <header>
        {#if icon}<div class="icon">{@render icon()}</div>{/if}
        <h2 id={titleId}>{title}</h2>
        {#if dismissible}
          <button type="button" class="icon-btn close" aria-label="Close" onclick={onclose}><X size={16} /></button>
        {/if}
      </header>
      <div class="body">
        {@render children()}
      </div>
      {#if actions}
        <footer class="actions">{@render actions()}</footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: var(--scrim);
    backdrop-filter: blur(6px);
    cursor: default;
  }
  .scrim:focus-visible {
    box-shadow: none;
  }
  .panel {
    position: relative;
    max-height: calc(100vh - 80px);
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    padding: 22px;
  }
  .panel:focus-visible {
    box-shadow: var(--shadow-lg);
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }
  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  h2 {
    flex: 1;
    font-size: 17px;
    font-weight: 650;
  }
  .close {
    margin: -6px -6px 0 0;
    align-self: flex-start;
  }
  .body {
    color: var(--text-muted);
    font-size: 13.5px;
    display: grid;
    gap: 12px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }
</style>
