<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    label,
    description,
    children,
    extra,
    stacked = false,
  }: {
    label: string;
    description?: string;
    /** The control, placed on the right. */
    children: Snippet;
    /** Optional content under the description (status chips, hints). */
    extra?: Snippet;
    /** Place the control below the text instead of beside it. */
    stacked?: boolean;
  } = $props();
</script>

<div class="row" class:stacked>
  <div class="text">
    <div class="label">{label}</div>
    {#if description}<p class="desc">{description}</p>{/if}
    {#if extra}<div class="extra">{@render extra()}</div>{/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 16px 0;
    border-top: 1px solid var(--border);
  }
  .row:first-child {
    border-top: none;
    padding-top: 4px;
  }
  .row:last-child {
    padding-bottom: 4px;
  }
  .row.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }
  .text {
    min-width: 0;
    flex: 1;
  }
  .label {
    font-weight: 600;
    font-size: 14px;
  }
  .desc {
    margin-top: 3px;
    color: var(--text-muted);
    font-size: 13px;
    max-width: 560px;
  }
  .extra {
    margin-top: 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
  }
</style>
