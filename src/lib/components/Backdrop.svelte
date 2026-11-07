<script lang="ts">
  import { onMount } from "svelte";
  import { prefersReducedMotion } from "$lib/motion";

  // Animated app backdrop: twisting translucent ribbons of fine lines that flow like a wave,
  // with a soft center glow and a few drifting sparkles, all in the accent color.
  // Drawn on a canvas behind the content (cards stay opaque on top).
  // Capped at 30 fps, paused while the window is hidden, one still frame under reduced motion.

  let canvas = $state<HTMLCanvasElement>();

  type Sheet = { base: number; amp: number; freq: number; speed: number; phase: number; lines: number; spread: number; alpha: number };
  const SHEETS: Sheet[] = [
    { base: 0.6, amp: 0.12, freq: 1.15, speed: 0.22, phase: 0.0, lines: 28, spread: 0.11, alpha: 0.09 },
    { base: 0.68, amp: 0.09, freq: 0.85, speed: -0.16, phase: 2.4, lines: 20, spread: 0.08, alpha: 0.065 },
  ];
  const SPARKS = Array.from({ length: 22 }, (_, i) => ({ seed: (i * 0.618034) % 1, sheet: i % 2, off: ((i * 0.37) % 1) - 0.5 }));

  onMount(() => {
    const el = canvas;
    const ctx = el?.getContext("2d");
    if (!el || !ctx) return;

    let w = 0;
    let h = 0;
    let rgb = "124, 92, 255";
    let light = false;
    let raf = 0;
    let last = 0;
    let frames = 0;
    let t = Math.random() * 100;

    function readTheme() {
      const root = document.documentElement;
      const raw = getComputedStyle(root).getPropertyValue("--accent").trim() || "#7c5cff";
      ctx!.fillStyle = "#7c5cff";
      ctx!.fillStyle = raw; // canvas normalizes any CSS color to #rrggbb
      const hex = String(ctx!.fillStyle);
      if (/^#[0-9a-f]{6}$/i.test(hex)) {
        rgb = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(", ");
      }
      light = root.dataset.theme === "light";
    }

    function resize() {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = el!.clientWidth;
      h = el!.clientHeight;
      el!.width = Math.max(1, Math.round(w * dpr));
      el!.height = Math.max(1, Math.round(h * dpr));
      ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    }

    // y (0..1) of line `f` (-0.5..0.5 across the sheet) at horizontal position u.
    function sheetY(s: Sheet, u: number, f: number): number {
      const k = s.freq * Math.PI * 2;
      const body = Math.sin(u * k + t * s.speed + s.phase + f * 0.6) * s.amp;
      // The sheet's width breathes along its length, so it reads as a twisting ribbon.
      const twist = Math.sin(u * k * 0.55 - t * s.speed * 0.7 + s.phase * 1.3);
      return s.base + (0.5 - u) * 0.12 + body + f * s.spread * twist;
    }

    function trace(s: Sheet, f: number, step: number) {
      ctx!.beginPath();
      for (let x = 0; x <= w + step; x += step) {
        const y = sheetY(s, x / w, f) * h;
        if (x === 0) ctx!.moveTo(x, y);
        else ctx!.lineTo(x, y);
      }
    }

    function draw() {
      ctx!.clearRect(0, 0, w, h);
      if (!w || !h) return;
      const strength = light ? 0.85 : 1;
      const step = Math.max(4, w / 140);
      ctx!.globalCompositeOperation = light ? "source-over" : "lighter";

      for (const s of SHEETS) {
        ctx!.lineWidth = 1;
        for (let li = 0; li < s.lines; li++) {
          const f = li / (s.lines - 1) - 0.5;
          trace(s, f, step);
          // Edges of the sheet are brighter, like light catching a folded ribbon.
          const edge = 0.55 + Math.abs(f) * 0.9;
          ctx!.strokeStyle = `rgba(${rgb}, ${(s.alpha * edge * strength).toFixed(3)})`;
          ctx!.stroke();
        }
        // Soft glow along the ribbon's center.
        trace(s, 0, step);
        ctx!.lineWidth = 34 * (0.8 + s.spread * 2);
        ctx!.strokeStyle = `rgba(${rgb}, ${(0.025 * strength).toFixed(3)})`;
        ctx!.stroke();
      }

      // A few sparkles drifting along the ribbons.
      for (const p of SPARKS) {
        const u = (p.seed + t * 0.018) % 1;
        const y = sheetY(SHEETS[p.sheet], u, p.off) * h;
        const tw = 0.5 + 0.5 * Math.sin(t * 1.3 + p.seed * 30);
        ctx!.fillStyle = `rgba(${rgb}, ${((0.15 + tw * 0.45) * strength).toFixed(3)})`;
        ctx!.beginPath();
        ctx!.arc(u * w, y, 1.2 * (0.6 + tw * 0.6), 0, Math.PI * 2);
        ctx!.fill();
      }
      ctx!.globalCompositeOperation = "source-over";
    }

    function loop(now: number) {
      if (prefersReducedMotion.current) {
        draw(); // settle on a still frame and stop
        return;
      }
      raf = requestAnimationFrame(loop);
      if (now - last < 1000 / 30) return;
      const dt = last ? Math.min(now - last, 100) / 1000 : 0;
      last = now;
      t += dt;
      if (++frames % 30 === 0) readTheme();
      draw();
    }

    function start() {
      cancelAnimationFrame(raf);
      readTheme();
      draw();
      if (prefersReducedMotion.current) return;
      last = 0;
      raf = requestAnimationFrame(loop);
    }

    const ro = new ResizeObserver(() => {
      resize();
      draw();
    });
    ro.observe(el);

    // Redraw immediately when the theme or accent changes.
    const mo = new MutationObserver(() => {
      readTheme();
      draw();
    });
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["style", "data-theme", "class"] });

    function onVisibility() {
      if (document.hidden) cancelAnimationFrame(raf);
      else start();
    }
    document.addEventListener("visibilitychange", onVisibility);

    resize();
    start();

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      mo.disconnect();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  });
</script>

<canvas bind:this={canvas} class="backdrop" aria-hidden="true"></canvas>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: -1;
    pointer-events: none;
  }
</style>
