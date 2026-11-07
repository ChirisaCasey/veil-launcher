<script lang="ts">
  // Renders /logo.svg (provided separately). Falls back to an inline mark until it exists.
  let { size = 32 }: { size?: number } = $props();
  let failed = $state(false);
  const uid = `veil-logo-${Math.random().toString(36).slice(2, 8)}`;
</script>

{#if !failed}
  <img
    src="/logo.svg"
    alt=""
    width={size}
    height={size}
    draggable="false"
    onerror={() => (failed = true)}
    style:width="{size}px"
    style:height="{size}px"
  />
{:else}
  <svg width={size} height={size} viewBox="0 0 48 48" aria-hidden="true">
    <defs>
      <linearGradient id="{uid}-g" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" style="stop-color: var(--accent)" />
        <stop offset="1" style="stop-color: var(--accent-solid-end)" />
      </linearGradient>
    </defs>
    <rect x="2" y="2" width="44" height="44" rx="13" fill="url(#{uid}-g)" />
    <path d="M13 14 L24 36 L35 14" fill="none" stroke="#fff" stroke-width="4.2" stroke-linecap="round" stroke-linejoin="round" />
    <path d="M17.5 14 H30.5" stroke="#fff" stroke-opacity="0.55" stroke-width="3" stroke-linecap="round" />
  </svg>
{/if}

<style>
  img {
    object-fit: contain;
    -webkit-user-drag: none;
  }
</style>
