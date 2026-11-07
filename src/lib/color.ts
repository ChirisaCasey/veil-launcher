/** Small color helpers used to derive accessible accent tokens (WCAG 2.x contrast). */

export type RGB = [number, number, number];

export function isHex(v: unknown): v is string {
  return typeof v === "string" && /^#[0-9a-f]{6}$/i.test(v);
}

export function parseHex(hex: string): RGB {
  const h = hex.replace("#", "");
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)) as RGB;
}

export function toHex([r, g, b]: RGB): string {
  return "#" + [r, g, b].map((c) => Math.round(Math.max(0, Math.min(255, c))).toString(16).padStart(2, "0")).join("");
}

export function luminance(rgb: RGB): number {
  const [r, g, b] = rgb.map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrast(a: string, b: string): number {
  const la = luminance(parseHex(a));
  const lb = luminance(parseHex(b));
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

/** Linear sRGB-space mix: t=0 returns a, t=1 returns b. */
export function mix(a: string, b: string, t: number): string {
  const A = parseHex(a);
  const B = parseHex(b);
  return toHex([A[0] + (B[0] - A[0]) * t, A[1] + (B[1] - A[1]) * t, A[2] + (B[2] - A[2]) * t]);
}

/** Mix `fg` toward `toward` in small steps until it reaches `target` contrast against `bg`. */
export function ensureContrast(fg: string, bg: string, target: number, toward: string): string {
  let out = fg;
  for (let t = 0; t <= 1.0001 && contrast(out, bg) < target; t += 0.04) {
    out = mix(fg, toward, t);
  }
  return out;
}

const WHITE = "#ffffff";
const INK = "#0b0b12";

export interface AccentTokens {
  /** Raw accent, for decoration (glows, rings, indicators). */
  accent: string;
  /** Fill for primary buttons; always AA with `onAccent`. */
  solid: string;
  solidHover: string;
  /** Second gradient stop; moves away from the text color so contrast only improves. */
  solidEnd: string;
  onAccent: string;
  /** Accent usable as text/icon color on the theme's surfaces (AA 4.5:1). */
  text: string;
}

export function accentTokens(accent: string, theme: "light" | "dark"): AccentTokens {
  const whiteOk = contrast(accent, WHITE) >= 3;
  let solid: string;
  let onAccent: string;
  let solidHover: string;
  let solidEnd: string;
  if (whiteOk) {
    solid = ensureContrast(accent, WHITE, 4.6, INK);
    onAccent = WHITE;
    solidHover = mix(solid, INK, 0.1);
    solidEnd = mix(solid, INK, 0.22);
  } else {
    solid = ensureContrast(accent, INK, 4.6, WHITE);
    onAccent = INK;
    solidHover = mix(solid, WHITE, 0.14);
    solidEnd = mix(solid, WHITE, 0.2);
  }
  // Check against the most demanding surface of each theme.
  const text =
    theme === "dark"
      ? ensureContrast(accent, "#1d1d2a", 4.6, WHITE)
      : ensureContrast(accent, "#ececf2", 4.6, INK);
  return { accent, solid, solidHover, solidEnd, onAccent, text };
}
