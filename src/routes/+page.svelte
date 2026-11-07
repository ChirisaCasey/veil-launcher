<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { initApp, nav, theme } from "$lib/stores";
  import { VIEWS } from "$lib/stores/navigation.svelte";
  import { ms } from "$lib/motion";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import Backdrop from "$lib/components/Backdrop.svelte";
  import Home from "$lib/views/Home.svelte";
  import Videos from "$lib/views/Videos.svelte";
  import Detection from "$lib/views/Detection.svelte";
  import Settings from "$lib/views/Settings.svelte";

  let scroller = $state<HTMLElement>();

  // Theme: resolved mode + accent tokens written to <html>, live.
  $effect(() => {
    theme.apply();
  });

  onMount(() => {
    // Deep link (e.g. index.html#settings) — handy for previews.
    if (location.hash.length > 1) nav.go(location.hash.slice(1));
    let cleanup: (() => void) | undefined;
    let disposed = false;
    void initApp().then((c) => {
      if (disposed) c();
      else cleanup = c;
    });
    return () => {
      disposed = true;
      cleanup?.();
    };
  });

  // Each view starts at the top.
  $effect(() => {
    void nav.view;
    if (scroller) scroller.scrollTop = 0;
  });

  function isEditable(el: EventTarget | null): boolean {
    return el instanceof HTMLElement && !!el.closest('input, textarea, select, [contenteditable="true"]');
  }

  function oncontextmenu(e: MouseEvent) {
    // Keep the native menu only where it's useful (text fields, copyable paths). Shift+right-click in dev for devtools.
    if (import.meta.env.DEV && e.shiftKey) return;
    const t = e.target;
    if (isEditable(t)) return;
    if (t instanceof HTMLElement && t.closest(".selectable") && window.getSelection()?.toString()) return;
    e.preventDefault();
  }

  function onkeydown(e: KeyboardEvent) {
    if (!e.ctrlKey || e.altKey || e.shiftKey || e.metaKey || isEditable(e.target)) return;
    const n = Number(e.key);
    if (n >= 1 && n <= VIEWS.length) {
      e.preventDefault();
      nav.go(VIEWS[n - 1]);
    }
  }
</script>

<svelte:window {oncontextmenu} {onkeydown} />

<div class="app">
  <Sidebar />
  <div class="main">
    <Backdrop />
    <TitleBar />
    <main class="content" bind:this={scroller}>
      <div class="views">
        {#key nav.view}
          <div
            class="view"
            in:fly={{ y: 12 * nav.direction, duration: ms(320), delay: ms(70), easing: cubicOut }}
            out:fade={{ duration: ms(90) }}
          >
            {#if nav.view === "home"}
              <Home />
            {:else if nav.view === "videos"}
              <Videos />
            {:else if nav.view === "detection"}
              <Detection />
            {:else}
              <Settings />
            {/if}
          </div>
        {/key}
      </div>
    </main>
  </div>
</div>

<Toasts />

<style>
  .app {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  .main {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    isolation: isolate;
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
  }
  .views {
    display: grid;
    max-width: 1080px;
    margin: 0 auto;
    padding: 8px 32px 36px;
  }
  .view {
    grid-area: 1 / 1;
    min-width: 0;
  }
</style>
