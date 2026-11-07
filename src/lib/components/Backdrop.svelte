<script lang="ts">
  import { onMount } from "svelte";
  import { prefersReducedMotion } from "$lib/motion";
  import type { BackdropStyle } from "$lib/api";

  // Animated app backdrop in the accent color, drawn on a canvas behind the content
  // (cards stay opaque on top). Also used at thumbnail size for the Settings picker.
  // Capped at 30 fps, paused while the window is hidden, one still frame under reduced motion.
  //
  // Dot styles share one renderer: each computes an intensity 0..1 per grid dot, which sets
  // the dot's size and opacity. "lines" draws thin flowing contour lines instead.

  let {
    variant = "waves",
    /** Thumbnail mode: denser grid relative to size, always animated. */
    compact = false,
  }: { variant?: BackdropStyle; compact?: boolean } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let current: BackdropStyle = "waves";
  let kick: (() => void) | undefined;

  $effect(() => {
    current = variant;
    kick?.();
  });

  const LEVELS = 14; // alpha buckets, so each frame is ~LEVELS fills instead of one per dot

  // ---------- intensity functions (u, v in 0..1; i, j grid indices; t seconds) ----------

  type Wave = { base: number; amp: number; freq: number; speed: number; phase: number; width: number; weight: number };
  const WAVES: Wave[] = [
    { base: 0.58, amp: 0.1, freq: 1.0, speed: 0.06, phase: 0.0, width: 0.09, weight: 0.8 },
    { base: 0.75, amp: 0.08, freq: 1.4, speed: -0.045, phase: 2.1, width: 0.07, weight: 1 },
    { base: 0.9, amp: 0.06, freq: 0.8, speed: 0.05, phase: 4.2, width: 0.06, weight: 0.9 },
  ];

  function waveCenter(wv: Wave, u: number, t: number): number {
    const k = wv.freq * Math.PI * 2;
    const s = Math.sin(u * k + t * wv.speed * 6 + wv.phase) * 0.65 + Math.sin(u * k * 0.5 - t * wv.speed * 4 + wv.phase * 1.7) * 0.35;
    return wv.base + (0.5 - u) * 0.14 + s * wv.amp; // slight rise to the right, like the logo
  }

  function cornerGlow(u: number, v: number): number {
    const dx = u - 0.9;
    const dy = v + 0.05;
    return Math.exp(-(dx * dx + dy * dy) * 6) * 0.35;
  }

  // Cheap deterministic per-dot hash in 0..1.
  function hash(i: number, j: number): number {
    const s = Math.sin(i * 127.1 + j * 311.7) * 43758.5453;
    return s - Math.floor(s);
  }

  type Field = (u: number, v: number, i: number, j: number, t: number, aspect: number) => number;

  type DotStyle = Exclude<BackdropStyle, "lines" | "ribbon" | "off">;

  // Twisting translucent sheets of fine lines (a handheld-console-menu-style wave).
  type Sheet = { base: number; amp: number; freq: number; speed: number; phase: number; lines: number; spread: number; alpha: number };
  const SHEETS: Sheet[] = [
    { base: 0.6, amp: 0.12, freq: 1.15, speed: 0.22, phase: 0.0, lines: 28, spread: 0.11, alpha: 0.09 },
    { base: 0.68, amp: 0.09, freq: 0.85, speed: -0.16, phase: 2.4, lines: 20, spread: 0.08, alpha: 0.065 },
  ];
  const SPARKS = Array.from({ length: 22 }, (_, i) => ({ seed: (i * 0.618034) % 1, sheet: i % 2, off: ((i * 0.37) % 1) - 0.5 }));

  const FIELDS: Record<DotStyle, { prep?: (u: number, t: number) => number[]; f: Field }> = {
    // Broad flowing bands of brighter dots.
    waves: {
      prep: (u, t) => WAVES.map((wv) => waveCenter(wv, u, t)),
      f: () => 0, // handled specially in draw (needs per-column centers)
    },
    // Concentric rings expanding from two slowly drifting points.
    ripple: {
      f: (u, v, _i, _j, t, aspect) => {
        let I = 0;
        const sources = [
          [0.78 + Math.sin(t * 0.07) * 0.12, 0.25 + Math.cos(t * 0.05) * 0.1],
          [0.2 + Math.cos(t * 0.06) * 0.1, 0.85 + Math.sin(t * 0.08) * 0.08],
        ];
        for (const [sx, sy] of sources) {
          const dx = (u - sx) * aspect;
          const dy = v - sy;
          const d = Math.sqrt(dx * dx + dy * dy);
          const ring = Math.pow(0.5 + 0.5 * Math.sin(d * 22 - t * 1.1), 6);
          I += ring * Math.exp(-d * 1.6) * 0.9;
        }
        return I;
      },
    },
    // Dots twinkle independently, with a slow larger-scale swell.
    shimmer: {
      f: (u, v, i, j, t) => {
        const h = hash(i, j);
        const tw = Math.pow(0.5 + 0.5 * Math.sin(t * (0.6 + h * 1.4) + h * 40), 10);
        const swell = 0.5 + 0.5 * Math.sin(u * 3.2 + v * 2.1 + t * 0.15);
        return tw * (0.35 + swell * 0.65) + cornerGlow(u, v) * 0.6;
      },
    },
    // A soft diagonal beam that sweeps across the grid.
    beam: {
      f: (u, v, _i, _j, t) => {
        const pos = ((t * 0.045) % 1.6) - 0.3; // sweeps left→right, then repeats
        const d = (u * 0.75 + v * 0.45) / 1.2 - pos;
        return Math.exp(-((d / 0.09) ** 2)) + Math.exp(-(((d + 0.18) / 0.05) ** 2)) * 0.35 + cornerGlow(u, v) * 0.5;
      },
    },
  };

  onMount(() => {
    const el = canvas;
    const ctx = el?.getContext("2d");
    if (!el || !ctx) return;

    let w = 0;
    let h = 0;
    let gap = 22;
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
      gap = compact ? Math.max(6, w / 22) : 22;
      const ox = ((w % gap) + gap) / 2;
      const oy = ((h % gap) + gap) / 2;
      xs = [];
      ys = [];
      for (let x = ox; x < w; x += gap) xs.push(x);
      for (let y = oy; y < h; y += gap) ys.push(y);
    }

    // y (0..1) of line `f` (-0.5..0.5 across the sheet) at horizontal position u.
    function sheetY(s: Sheet, u: number, f: number): number {
      const k = s.freq * Math.PI * 2;
      const body = Math.sin(u * k + t * s.speed + s.phase + f * 0.6) * s.amp;
      // The sheet's width breathes along its length, so it reads as a twisting ribbon.
      const twist = Math.sin(u * k * 0.55 - t * s.speed * 0.7 + s.phase * 1.3);
      return s.base + (0.5 - u) * 0.12 + body + f * s.spread * twist;
    }

    function drawRibbon() {
      const light = document.documentElement.dataset.theme === "light";
      const step = Math.max(4, w / 140);
      ctx!.globalCompositeOperation = light ? "source-over" : "lighter";
      ctx!.lineWidth = compact ? 0.7 : 1;
      for (const s of SHEETS) {
        const n = compact ? Math.round(s.lines * 0.5) : s.lines;
        for (let li = 0; li < n; li++) {
          const f = li / (n - 1) - 0.5;
          ctx!.beginPath();
          for (let x = 0; x <= w + step; x += step) {
            const y = sheetY(s, x / w, f) * h;
            if (x === 0) ctx!.moveTo(x, y);
            else ctx!.lineTo(x, y);
          }
          // Edges of the sheet are brighter, like light catching a folded ribbon.
          const edge = 0.55 + Math.abs(f) * 0.9;
          ctx!.strokeStyle = `rgba(${rgb}, ${(s.alpha * edge * strength).toFixed(3)})`;
          ctx!.stroke();
        }
        // Soft glow along the ribbon's center.
        ctx!.beginPath();
        for (let x = 0; x <= w + step; x += step) {
          const y = sheetY(s, x / w, 0) * h;
          if (x === 0) ctx!.moveTo(x, y);
          else ctx!.lineTo(x, y);
        }
        ctx!.lineWidth = (compact ? 10 : 34) * (0.8 + s.spread * 2);
        ctx!.strokeStyle = `rgba(${rgb}, ${(0.025 * strength).toFixed(3)})`;
        ctx!.stroke();
        ctx!.lineWidth = compact ? 0.7 : 1;
      }
      // A few sparkles drifting along the ribbons.
      for (const p of SPARKS) {
        const u = (p.seed + t * 0.018) % 1;
        const y = sheetY(SHEETS[p.sheet], u, p.off) * h;
        const tw = 0.5 + 0.5 * Math.sin(t * 1.3 + p.seed * 30);
        const r = (compact ? 0.7 : 1.2) * (0.6 + tw * 0.6);
        ctx!.fillStyle = `rgba(${rgb}, ${(0.15 + tw * 0.45) * strength})`;
        ctx!.beginPath();
        ctx!.arc(u * w, y, r, 0, Math.PI * 2);
        ctx!.fill();
      }
      ctx!.globalCompositeOperation = "source-over";
    }

    function drawDots(style: DotStyle) {
      const buckets: number[][] = Array.from({ length: LEVELS }, () => []);
      const aspect = w / Math.max(1, h);
      const field = FIELDS[style];
      for (let i = 0; i < xs.length; i++) {
        const x = xs[i];
        const u = x / w;
        const centers = style === "waves" ? field.prep!(u, t) : null;
        for (let j = 0; j < ys.length; j++) {
          const y = ys[j];
          const v = y / h;
          let I: number;
          if (centers) {
            I = cornerGlow(u, v);
            for (let k = 0; k < WAVES.length; k++) {
              const d = (v - centers[k]) / WAVES[k].width;
              I += Math.exp(-d * d) * WAVES[k].weight;
            }
          } else {
            I = field.f(u, v, i, j, t, aspect);
          }
          const level = Math.min(LEVELS - 1, Math.floor(Math.min(I, 1) * (LEVELS - 1)));
          buckets[level].push(x, y);
        }
      }
      const rScale = compact ? Math.max(0.6, gap / 14) : 1;
      for (let l = 0; l < LEVELS; l++) {
        const pts = buckets[l];
        if (!pts.length) continue;
        const k = l / (LEVELS - 1);
        const alpha = (0.1 + k * 0.5) * strength;
        const r = (0.5 + k * 0.6) * rScale;
        ctx!.fillStyle = `rgba(${rgb}, ${alpha.toFixed(3)})`;
        ctx!.beginPath();
        for (let p = 0; p < pts.length; p += 2) {
          ctx!.moveTo(pts[p] + r, pts[p + 1]);
          ctx!.arc(pts[p], pts[p + 1], r, 0, Math.PI * 2);
        }
        ctx!.fill();
      }
    }

    // Thin flowing contour lines (a "topographic veil").
    function drawLines() {
      const count = compact ? 9 : 14;
      const step = Math.max(4, w / 120);
      ctx!.lineWidth = compact ? 0.8 : 1;
      for (let n = 0; n < count; n++) {
        const f = n / (count - 1);
        const base = 0.35 + f * 0.62;
        const amp = 0.05 + f * 0.04;
        const ph = n * 0.55;
        ctx!.beginPath();
        for (let x = 0; x <= w + step; x += step) {
          const u = x / w;
          const y =
            (base + (0.5 - u) * 0.16 + Math.sin(u * 5.5 + t * 0.32 + ph) * amp * 0.65 + Math.sin(u * 2.3 - t * 0.21 + ph * 1.6) * amp * 0.35) * h;
          if (x === 0) ctx!.moveTo(x, y);
          else ctx!.lineTo(x, y);
        }
        const a = (0.06 + Math.pow(f, 1.4) * 0.22) * strength;
        ctx!.strokeStyle = `rgba(${rgb}, ${a.toFixed(3)})`;
        ctx!.stroke();
      }
    }

    function draw() {
      ctx!.clearRect(0, 0, w, h);
      if (!xs.length || current === "off") return;
      if (current === "lines") drawLines();
      else if (current === "ribbon") drawRibbon();
      else drawDots(current);
    }

    const still = () => prefersReducedMotion.current && !compact;

    function loop(now: number) {
      if (still() || current === "off") {
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
      if (still() || current === "off") return;
      last = 0;
      raf = requestAnimationFrame(loop);
    }
    kick = start;

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
      kick = undefined;
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
