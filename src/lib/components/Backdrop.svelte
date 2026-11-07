<script lang="ts">
  import { onMount } from "svelte";
  import { prefersReducedMotion } from "$lib/motion";

  // Animated app backdrop: a dot grid in the accent color. Slow "veil" waves (echoing the
  // logo's ribbons) drift across it, brightening and enlarging the dots they pass over.
  // Drawn on a canvas behind the content; cards stay opaque on top.
  // Capped at 30 fps, paused while the window is hidden, one still frame under reduced motion.

  let canvas = $state<HTMLCanvasElement>();

  const GAP = 22; // grid spacing (px)
  const LEVELS = 14; // alpha buckets, so each frame is ~LEVELS fills instead of one per dot

  // Each wave is a band of dots that lights up around a flowing centerline.
  type Wave = { base: number; amp: number; freq: number; speed: number; phase: number; width: number; weight: number };
  const WAVES: Wave[] = [
    { base: 0.58, amp: 0.1, freq: 1.0, speed: 0.06, phase: 0.0, width: 0.09, weight: 0.8 },
    { base: 0.75, amp: 0.08, freq: 1.4, speed: -0.045, phase: 2.1, width: 0.07, weight: 1 },
    { base: 0.9, amp: 0.06, freq: 0.8, speed: 0.05, phase: 4.2, width: 0.06, weight: 0.9 },
  ];

  onMount(() => {
    const el = canvas;
    const ctx = el?.getContext("2d");
    if (!el || !ctx) return;

    let w = 0;
    let h = 0;
    let xs: number[] = [];
    let ys: number[] = [];
    let rgb = "124, 92, 255";
    let strength = 1;
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
      strength = root.dataset.theme === "light" ? 0.85 : 1;
    }

    function resize() {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = el!.clientWidth;
      h = el!.clientHeight;
      el!.width = Math.max(1, Math.round(w * dpr));
      el!.height = Math.max(1, Math.round(h * dpr));
      ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
      // Centered grid.
      const ox = ((w % GAP) + GAP) / 2;
      const oy = ((h % GAP) + GAP) / 2;
      xs = [];
      ys = [];
      for (let x = ox; x < w; x += GAP) xs.push(x);
      for (let y = oy; y < h; y += GAP) ys.push(y);
    }

    // Wave centerline (0..1 of height) at horizontal position u.
    function center(wv: Wave, u: number): number {
      const k = wv.freq * Math.PI * 2;
      const s = Math.sin(u * k + t * wv.speed * 6 + wv.phase) * 0.65 + Math.sin(u * k * 0.5 - t * wv.speed * 4 + wv.phase * 1.7) * 0.35;
      return wv.base + (0.5 - u) * 0.14 + s * wv.amp; // slight rise to the right, like the logo
    }

    function draw() {
      ctx!.clearRect(0, 0, w, h);
      if (!xs.length) return;

      // Bucket dots by intensity, then fill each bucket in one pass.
      const buckets: number[][] = Array.from({ length: LEVELS }, () => []);
      for (const x of xs) {
        const u = x / w;
        const centers = WAVES.map((wv) => center(wv, u));
        for (const y of ys) {
          const v = y / h;
          let I = 0;
          for (let i = 0; i < WAVES.length; i++) {
            const d = (v - centers[i]) / WAVES[i].width;
            I += Math.exp(-d * d) * WAVES[i].weight;
          }
          // Faint ambient lift toward the top-right corner.
          const dx = u - 0.9;
          const dy = v + 0.05;
          I += Math.exp(-(dx * dx + dy * dy) * 6) * 0.35;
          const level = Math.min(LEVELS - 1, Math.floor(Math.min(I, 1) * (LEVELS - 1)));
          buckets[level].push(x, y);
        }
      }

      for (let l = 0; l < LEVELS; l++) {
        const pts = buckets[l];
        if (!pts.length) continue;
        const k = l / (LEVELS - 1);
        // Small dots; brighter and slightly larger on a wave crest.
        const alpha = (0.1 + k * 0.5) * strength;
        const r = 0.5 + k * 0.6;
        ctx!.fillStyle = `rgba(${rgb}, ${alpha.toFixed(3)})`;
        ctx!.beginPath();
        for (let i = 0; i < pts.length; i += 2) {
          ctx!.moveTo(pts[i] + r, pts[i + 1]);
          ctx!.arc(pts[i], pts[i + 1], r, 0, Math.PI * 2);
        }
        ctx!.fill();
      }
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
      if (prefersReducedMotion.current) {
        draw();
        return;
      }
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
