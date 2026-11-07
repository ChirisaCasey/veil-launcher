<script lang="ts">
  import { settings, toasts } from "$lib/stores";
  import { REPO_URL, VAL_PATCHER_URL } from "$lib/constants";
  import { openExternal } from "$lib/platform";
  import Logo from "$lib/components/Logo.svelte";
  import ShieldCheck from "@lucide/svelte/icons/shield-check";
  import Repeat from "@lucide/svelte/icons/repeat";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import ArrowRight from "@lucide/svelte/icons/arrow-right";
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import SquareTerminal from "@lucide/svelte/icons/square-terminal";
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import EyeOff from "@lucide/svelte/icons/eye-off";

  const STEPS = [
    {
      icon: ShieldCheck,
      title: "Riot checks files",
      body: "You launch VALORANT. The Riot Client verifies the game files as usual, untouched.",
    },
    {
      icon: Repeat,
      title: "Veil swaps your video",
      body: "Once the check is done, Veil backs up the menu video and puts yours in its place.",
    },
    {
      icon: RotateCcw,
      title: "Original restored on exit",
      body: "When the game closes, the original goes back, so the next normal launch needs no repair.",
    },
  ];

  async function open(url: string) {
    try {
      await openExternal(url);
    } catch (e) {
      toasts.error(e, "Couldn't open link");
    }
  }
</script>

<div class="about">
  <section class="intro">
    <div class="logo-wrap">
      <span class="logo-glow"></span>
      <Logo size={84} />
    </div>
    <h1>Veil Launcher</h1>
    <p class="version">Version {settings.app?.version ?? "…"}</p>
    <p class="tagline">Your own VALORANT menu background, swapped in safely after Riot's file check and put back when you're done.</p>
  </section>

  <section class="how" aria-labelledby="how-title">
    <h2 id="how-title" class="sr-only">How it works</h2>
    <ol class="steps">
      {#each STEPS as step, i (step.title)}
        {@const Icon = step.icon}
        <li class="step card" style:--i={i}>
          <span class="step-num">{i + 1}</span>
          <span class="step-icon"><Icon size={22} /></span>
          <h3>{step.title}</h3>
          <p>{step.body}</p>
        </li>
        {#if i < STEPS.length - 1}
          <li class="arrow" aria-hidden="true"><ArrowRight size={18} /></li>
        {/if}
      {/each}
    </ol>
  </section>

  <section class="note callout accent">
    <EyeOff size={18} />
    <div>
      <strong>No injection, no overlay — just a file swap.</strong>
      <p>Veil never touches the game process or its memory. It only copies a video file in the game's Menu folder, keeps a backup, and restores it.</p>
    </div>
  </section>

  <section class="links">
    <button type="button" class="link card" onclick={() => open(REPO_URL)}>
      <span class="link-icon"><GitBranch size={20} /></span>
      <span class="link-text">
        <span class="link-title">Source code on GitHub</span>
        <span class="link-sub">Releases, issues and the README</span>
      </span>
      <ExternalLink size={16} />
    </button>
    <button type="button" class="link card" onclick={() => open(VAL_PATCHER_URL)}>
      <span class="link-icon"><SquareTerminal size={20} /></span>
      <span class="link-text">
        <span class="link-title">Prefer a script? Try Val-Patcher</span>
        <span class="link-sub">The lite .bat version: easy to read, easy to see what it does</span>
      </span>
      <ExternalLink size={16} />
    </button>
  </section>

  <p class="disclaimer">
    Veil Launcher is an independent project and is not affiliated with, endorsed by, or sponsored by Riot Games.
    VALORANT and Riot Games are trademarks or registered trademarks of Riot Games, Inc.
  </p>
</div>

<style>
  .about {
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-width: 860px;
    margin: 0 auto;
    padding-top: 8px;
  }
  .intro {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }
  .logo-wrap {
    position: relative;
    display: grid;
    place-items: center;
    margin-bottom: 14px;
  }
  .logo-wrap > :global(*) {
    grid-area: 1 / 1;
    position: relative;
  }
  .logo-glow {
    width: 150px;
    height: 150px;
    border-radius: 50%;
    background: radial-gradient(circle, color-mix(in srgb, var(--accent) 40%, transparent), transparent 65%);
    filter: blur(8px);
    animation: glow 4s ease-in-out infinite;
  }
  @keyframes glow {
    50% {
      transform: scale(1.12);
      opacity: 0.7;
    }
  }
  h1 {
    font-size: 30px;
    font-weight: 750;
    letter-spacing: -0.03em;
  }
  .version {
    margin-top: 4px;
    font-size: 13px;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }
  .tagline {
    margin-top: 10px;
    max-width: 520px;
    color: var(--text-muted);
    font-size: 14.5px;
  }

  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: 1fr auto 1fr auto 1fr;
    align-items: stretch;
    gap: 10px;
  }
  .step {
    padding: 18px;
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    animation: rise 0.5s var(--ease-out) backwards;
    animation-delay: calc(var(--i) * 90ms + 80ms);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
  }
  .step-num {
    position: absolute;
    top: 14px;
    right: 16px;
    font-size: 12px;
    font-weight: 700;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }
  .step-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 13px;
    background: var(--accent-gradient);
    color: var(--on-accent);
    box-shadow: 0 8px 20px -8px color-mix(in srgb, var(--accent) 80%, transparent);
    margin-bottom: 14px;
  }
  .step h3 {
    font-size: 15px;
    font-weight: 650;
  }
  .step p {
    margin-top: 6px;
    font-size: 13px;
    color: var(--text-muted);
  }
  .arrow {
    display: grid;
    place-items: center;
    color: var(--text-faint);
  }

  .note strong {
    display: block;
    font-size: 14px;
  }
  .note p {
    margin-top: 2px;
    color: var(--text-muted);
  }

  .links {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    text-align: left;
    border-radius: var(--radius);
    box-shadow: var(--shadow-sm);
    color: var(--text-faint);
    transition: border-color 0.15s ease, transform 0.2s var(--ease-out);
  }
  .link:hover {
    border-color: var(--accent-border);
    transform: translateY(-1px);
  }
  .link:focus-visible {
    border-radius: var(--radius);
  }
  .link-icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 11px;
    background: var(--surface-3);
    color: var(--text);
  }
  .link-text {
    flex: 1;
    min-width: 0;
    display: grid;
  }
  .link-title {
    font-weight: 600;
    color: var(--text);
  }
  .link-sub {
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .disclaimer {
    font-size: 12px;
    color: var(--text-faint);
    text-align: center;
    max-width: 620px;
    margin: 4px auto 0;
  }

  @media (max-width: 1000px) {
    .steps {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
    .arrow {
      display: none;
    }
    .step {
      padding: 16px;
    }
  }
</style>
