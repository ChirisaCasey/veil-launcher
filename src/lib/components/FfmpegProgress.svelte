<script lang="ts">
  import { slide } from "svelte/transition";
  import { ffmpeg } from "$lib/stores";
  import { ms } from "$lib/motion";
  import ProgressBar from "./ProgressBar.svelte";
</script>

{#if ffmpeg.installing || ffmpeg.error}
  <div class="progress" transition:slide={{ duration: ms(200) }}>
    <div class="row">
      <span class="stage">{ffmpeg.error ? "Download failed" : ffmpeg.stage || "Starting download"}</span>
      {#if !ffmpeg.error}<span class="pct">{Math.round(ffmpeg.percent)}%</span>{/if}
    </div>
    <ProgressBar value={ffmpeg.percent} label="ffmpeg download" tone={ffmpeg.error ? "danger" : "accent"} />
    {#if ffmpeg.error}<p class="err selectable">{ffmpeg.error}</p>{/if}
  </div>
{/if}

<style>
  .progress {
    display: grid;
    gap: 8px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .pct {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--text);
  }
  .err {
    font-size: 12.5px;
    color: var(--danger);
  }
</style>
