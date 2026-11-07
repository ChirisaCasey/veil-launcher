<script lang="ts">
  import { onMount } from "svelte";
  import { api, isTauri } from "$lib/api";
  import { minimizeWindow, toggleMaximizeWindow, watchMaximized } from "$lib/platform";
  import { toasts } from "$lib/stores";

  let maximized = $state(false);

  onMount(() => watchMaximized((m) => (maximized = m)));

  async function close() {
    try {
      await api.hideToTray();
    } catch (e) {
      toasts.error(e, "Couldn't hide to tray");
    }
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="title" data-tauri-drag-region>
    {#if !isTauri}
      <span class="chip info preview" data-tauri-drag-region title="Running in a browser with a simulated backend">Browser preview</span>
    {/if}
  </div>
  <div class="controls">
    <button type="button" class="ctl" aria-label="Minimize" title="Minimize" onclick={minimizeWindow}>
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" stroke="currentColor" stroke-width="1" /></svg>
    </button>
    <button
      type="button"
      class="ctl"
      aria-label={maximized ? "Restore" : "Maximize"}
      title={maximized ? "Restore" : "Maximize"}
      onclick={toggleMaximizeWindow}
    >
      {#if maximized}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="0.5" y="2.5" width="7" height="7" rx="1" />
          <path d="M2.5 2.5V1.5a1 1 0 0 1 1-1h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-1" />
        </svg>
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="0.5" y="0.5" width="9" height="9" rx="1" />
        </svg>
      {/if}
    </button>
    <button type="button" class="ctl close" aria-label="Hide to tray" title="Hide to tray" onclick={close}>
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="currentColor" stroke-width="1.1" /></svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: var(--titlebar-h);
    display: flex;
    align-items: stretch;
    justify-content: space-between;
    flex: none;
  }
  .title {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    padding-left: 32px;
    font-size: 12.5px;
    font-weight: 550;
    color: var(--text-faint);
    min-width: 0;
  }
  .preview {
    height: 20px;
    font-size: 11px;
  }
  .controls {
    display: flex;
    flex: none;
  }
  .ctl {
    width: 46px;
    display: grid;
    place-items: center;
    color: var(--text-muted);
    transition: background-color 0.12s ease, color 0.12s ease;
    border-radius: 0;
  }
  .ctl:hover {
    background: var(--surface-3);
    color: var(--text);
  }
  .ctl.close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .ctl:focus-visible {
    border-radius: 0;
    box-shadow: inset 0 0 0 2px var(--accent);
  }
</style>
