<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, subtitle, actions }: { title: string; subtitle?: string; actions?: Snippet } = $props();
</script>

<header class="view-header">
  <div class="text">
    <h1>{title}</h1>
    {#if subtitle}<p>{subtitle}</p>{/if}
  </div>
  {#if actions}
    <div class="actions">{@render actions()}</div>
  {/if}
</header>

<style>
  .view-header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 22px;
  }
  .text {
    min-width: 0;
  }
  h1 {
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.025em;
  }
  p {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 14px;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex: none;
  }
</style>
