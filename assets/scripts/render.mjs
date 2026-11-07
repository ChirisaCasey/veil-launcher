// Render brand SVGs to PNG with @resvg/resvg-js.
// (Regenerate the SVGs first with `node assets/scripts/build-brand.cjs` if you
// changed assets/icon.template.svg or the ribbon parameters.)
//
// resvg is intentionally NOT a project dependency. Install it somewhere
// temporary and point NODE_PATH at it, e.g. (PowerShell):
//
//   npm i --prefix $env:TEMP\veil-tools @resvg/resvg-js
//   $env:NODE_PATH = "$env:TEMP\veil-tools\node_modules"
//   node assets/scripts/render.mjs
//
// Then regenerate the Tauri icon set from the repo root:
//   npx tauri icon assets/icon-1024.png
//
// Usage: node assets/scripts/render.mjs [input.svg output.png width] ...
// With no arguments, renders the default set below.
import { createRequire } from "node:module";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// ESM ignores NODE_PATH, so resolve through CommonJS require.
const require = createRequire(import.meta.url);
const { Resvg } = require("@resvg/resvg-js");

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

const defaults = [
  ["assets/icon.svg", "assets/icon-1024.png", 1024],
  ["assets/icon.svg", "static/favicon.png", 64],
  ["assets/banner.svg", "assets/banner.png", 1280],
];

function render(input, output, width) {
  const svg = readFileSync(resolve(root, input), "utf8");
  const png = new Resvg(svg, {
    fitTo: { mode: "width", value: Number(width) },
    font: { loadSystemFonts: true, defaultFontFamily: "Segoe UI" },
    shapeRendering: 2,
    textRendering: 1,
  })
    .render()
    .asPng();
  writeFileSync(resolve(root, output), png);
  console.log(`${input} -> ${output} (${width}px, ${png.length} bytes)`);
}

const args = process.argv.slice(2);
if (args.length) {
  for (let i = 0; i + 2 < args.length + 1; i += 3) render(args[i], args[i + 1], args[i + 2]);
} else {
  for (const job of defaults) render(...job);
}
