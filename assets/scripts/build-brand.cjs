// Generates the Veil Launcher brand SVGs from source parameters.
//
//   assets/icon.template.svg  ->  assets/icon.svg   (app icon, 1024 squircle tile)
//   assets/icon.svg           ->  static/logo.svg   (mark only, transparent)
//   both                      ->  assets/banner.svg (README banner)
//
// No dependencies. Run from the repo root:
//   node assets/scripts/build-brand.cjs
// then render PNGs with assets/scripts/render.mjs (see its header).
const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const read = (p) => fs.readFileSync(path.join(root, p), "utf8").replace(/^﻿/, "");
const write = (p, s) => {
  fs.writeFileSync(path.join(root, p), s);
  console.log(`wrote ${p} (${s.length} bytes)`);
};

// ---------- Veil ribbons ----------
// Each veil is a tapered ribbon around a rising sine centerline.
const RIBBONS = {
  back:  { x0: 120, x1: 900, yc: 400, slope: -0.36, amp: 70, period: 820, phase: 0.1, thick: 270 },
  mid:   { x0: 115, x1: 905, yc: 520, slope: -0.36, amp: 80, period: 820, phase: 0.5, thick: 285 },
  front: { x0: 120, x1: 910, yc: 640, slope: -0.36, amp: 90, period: 820, phase: 0.9, thick: 300 },
};
const SHAPE = { taper: 0.9, skew: 0.4, warp: 1.5, samples: 26 };

function ribbon({ x0, x1, yc, slope, amp, period, phase, thick }) {
  const { taper, skew, warp, samples } = SHAPE;
  const top = [], bot = [];
  for (let i = 0; i <= samples; i++) {
    const u = (1 - Math.cos((Math.PI * i) / samples)) / 2; // denser near the tips
    const x = x0 + (x1 - x0) * u;
    const y = yc + slope * (x - (x0 + x1) / 2) + amp * Math.sin((2 * Math.PI * (x - x0)) / period + phase);
    const t = thick * Math.pow(Math.sin(Math.PI * Math.pow(u, warp)), taper);
    const k = 0.5 + skew * (u - 0.5);
    top.push([x, y - t * k]);
    bot.push([x, y + t * (1 - k)]);
  }
  return smoothClosed(top.concat(bot.reverse().slice(1, -1)));
}

// Closed Catmull-Rom spline through the points, as cubic Beziers.
function smoothClosed(p) {
  const f = (n) => (Math.round(n * 10) / 10).toString();
  const n = p.length;
  let d = `M${f(p[0][0])} ${f(p[0][1])}`;
  for (let i = 0; i < n; i++) {
    const p0 = p[(i - 1 + n) % n], p1 = p[i], p2 = p[(i + 1) % n], p3 = p[(i + 2) % n];
    const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    d += `C${f(c1[0])} ${f(c1[1])} ${f(c2[0])} ${f(c2[1])} ${f(p2[0])} ${f(p2[1])}`;
  }
  return d + "Z";
}

// ---------- icon.svg ----------
let icon = read("assets/icon.template.svg");
for (const [k, v] of Object.entries(RIBBONS)) icon = icon.replaceAll(`{{${k}}}`, ribbon(v));
write("assets/icon.svg", icon);

// ---------- logo.svg (no tile, glow or rim) ----------
let logo = icon
  .replace('width="1024" height="1024" viewBox="0 0 1024 1024"', 'width="256" height="256" viewBox="92 92 840 840"')
  .replace(/\s*<path id="tile"[^>]*\/>\s*<path[^>]*url\(#glow\)[^>]*\/>\s*<path[^>]*url\(#rim\)[^>]*\/>/, "")
  .replace(/\s*<linearGradient id="base"[\s\S]*?<\/linearGradient>/, "")
  .replace(/\s*<radialGradient id="glow"[\s\S]*?<\/radialGradient>/, "")
  .replace(/\s*<linearGradient id="rim"[\s\S]*?<\/linearGradient>/, "");
write("static/logo.svg", logo);

// ---------- banner.svg ----------
const nestedIcon = icon
  .replace('<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">',
           '<svg x="96" y="72" width="256" height="256" viewBox="0 0 1024 1024">')
  .trim();
const deco = logo
  .replace(/<svg [^>]*>/, '<svg x="960" y="-100" width="560" height="560" viewBox="92 92 840 840">')
  .replace(/id="([^"]+)"/g, 'id="d-$1"')
  .replace(/url\(#([^)]+)\)/g, "url(#d-$1)")
  .trim();

const banner = `<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="400" viewBox="0 0 1280 400">
  <defs>
    <linearGradient id="b-bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#151127"/>
      <stop offset="1" stop-color="#08070e"/>
    </linearGradient>
    <radialGradient id="b-glow1" gradientUnits="userSpaceOnUse" cx="224" cy="200" r="440">
      <stop offset="0" stop-color="#7c5cff" stop-opacity="0.42"/>
      <stop offset="1" stop-color="#7c5cff" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="b-glow2" gradientUnits="userSpaceOnUse" cx="1180" cy="0" r="460">
      <stop offset="0" stop-color="#ff4fc8" stop-opacity="0.18"/>
      <stop offset="1" stop-color="#ff4fc8" stop-opacity="0"/>
    </radialGradient>
    <radialGradient id="b-glow3" gradientUnits="userSpaceOnUse" cx="940" cy="420" r="400">
      <stop offset="0" stop-color="#36dcff" stop-opacity="0.14"/>
      <stop offset="1" stop-color="#36dcff" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="b-word" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#9d82ff"/>
      <stop offset="0.55" stop-color="#e266ff"/>
      <stop offset="1" stop-color="#5fdcff"/>
    </linearGradient>
    <linearGradient id="b-line" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#7c5cff" stop-opacity="0.9"/>
      <stop offset="1" stop-color="#7c5cff" stop-opacity="0"/>
    </linearGradient>
    <pattern id="b-grid" width="32" height="32" patternUnits="userSpaceOnUse">
      <path d="M32 0H0V32" fill="none" stroke="#ffffff" stroke-opacity="0.035" stroke-width="1"/>
    </pattern>
  </defs>

  <rect width="1280" height="400" fill="url(#b-bg)"/>
  <rect width="1280" height="400" fill="url(#b-grid)"/>
  <rect width="1280" height="400" fill="url(#b-glow1)"/>
  <rect width="1280" height="400" fill="url(#b-glow2)"/>
  <rect width="1280" height="400" fill="url(#b-glow3)"/>

  <g opacity="0.17">${deco}</g>

  ${nestedIcon}

  <g font-family="Segoe UI, Inter, system-ui, -apple-system, Helvetica, Arial, sans-serif">
    <text x="404" y="196" font-size="96" font-weight="700" letter-spacing="-2" fill="#ffffff">Veil <tspan fill="url(#b-word)">Launcher</tspan></text>
    <rect x="408" y="226" width="420" height="3" rx="1.5" fill="url(#b-line)"/>
    <text x="408" y="282" font-size="34" font-weight="400" fill="#c9c3e6">Your VALORANT menu. Your vibe.</text>
  </g>
</svg>
`;
write("assets/banner.svg", banner);
