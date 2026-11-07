<script lang="ts">
  import { fly, fade } from "svelte/transition";
  import { flip } from "svelte/animate";
  import { cubicOut } from "svelte/easing";
  import { toasts } from "$lib/stores";
  import { ms } from "$lib/motion";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Info from "@lucide/svelte/icons/info";
  import X from "@lucide/svelte/icons/x";

  const ICONS = { success: CircleCheck, error: CircleAlert, warning: TriangleAlert, info: Info };
</script>

<div class="toasts" aria-live="polite" aria-relevant="additions">
  {#each toasts.items as t (t.id)}
    {@const Icon = ICONS[t.level]}
    <div
      class="toast {t.level}"
      role={t.level === "error" ? "alert" : "status"}
      animate:flip={{ duration: ms(220) }}
      in:fly={{ x: 24, duration: ms(260), easing: cubicOut }}
      out:fade={{ duration: ms(160) }}
      onmouseenter={() => toasts.hold(t.id)}
      onmouseleave={() => toasts.release(t.id)}
    >
      <span class="icon"><Icon size={18} /></span>
      <div class="text">
        {#if t.title}<div class="title">{t.title}</div>{/if}
        <div class="msg selectable">{t.message}</div>
      </div>
      <button type="button" class="icon-btn dismiss" aria-label="Dismiss" onclick={() => toasts.dismiss(t.id)}>
        <X size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 60;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(380px, calc(100vw - 40px));
    pointer-events: none;
  }
  .toast {
    --tone: var(--info);
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 10px 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-glass);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-lg);
    position: relative;
    overflow: hidden;
  }
  .toast::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--tone);
  }
  .success {
    --tone: var(--success);
  }
  .error {
    --tone: var(--danger);
  }
  .warning {
    --tone: var(--warning);
  }
  .icon {
    color: var(--tone);
    margin-top: 1px;
    flex: none;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-weight: 650;
    font-size: 13.5px;
  }
  .msg {
    font-size: 13px;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  .title + .msg {
    margin-top: 1px;
  }
  .dismiss {
    width: 26px;
    height: 26px;
    flex: none;
  }
</style>
