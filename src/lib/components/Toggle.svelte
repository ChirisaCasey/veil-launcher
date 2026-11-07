<script lang="ts">
  let {
    checked,
    onchange,
    disabled = false,
    label,
    busy = false,
    size = "md",
  }: {
    checked: boolean;
    onchange: (value: boolean) => void;
    disabled?: boolean;
    /** Accessible name (visually hidden). */
    label: string;
    busy?: boolean;
    size?: "md" | "lg";
  } = $props();
</script>

<button
  type="button"
  role="switch"
  class="switch {size}"
  class:on={checked}
  class:busy
  aria-checked={checked}
  aria-label={label}
  aria-busy={busy}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="thumb"></span>
</button>

<style>
  .switch {
    --w: 40px;
    --h: 22px;
    --pad: 3px;
    position: relative;
    flex: none;
    width: var(--w);
    height: var(--h);
    border-radius: 999px;
    background: var(--track);
    border: 1px solid var(--border-strong);
    transition: background-color 0.2s ease, border-color 0.2s ease, box-shadow 0.2s ease;
  }
  .switch.lg {
    --w: 48px;
    --h: 26px;
  }
  .switch.on {
    background: var(--accent-solid);
    border-color: transparent;
    box-shadow: 0 0 16px -4px color-mix(in srgb, var(--accent) 70%, transparent);
  }
  .switch:disabled {
    opacity: 0.5;
  }
  .switch:focus-visible {
    box-shadow: var(--focus-ring);
  }
  .thumb {
    position: absolute;
    top: calc(var(--pad) - 1px);
    left: calc(var(--pad) - 1px);
    width: calc(var(--h) - var(--pad) * 2);
    height: calc(var(--h) - var(--pad) * 2);
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
    transition: transform 0.25s var(--ease-spring), width 0.15s ease;
  }
  .on .thumb {
    transform: translateX(calc(var(--w) - var(--h)));
  }
  .switch:active:not(:disabled) .thumb {
    width: calc(var(--h) - var(--pad) * 2 + 4px);
  }
  .on:active:not(:disabled) .thumb {
    transform: translateX(calc(var(--w) - var(--h) - 4px));
  }
  .busy .thumb {
    animation: pulse 0.9s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.6;
    }
  }
</style>
