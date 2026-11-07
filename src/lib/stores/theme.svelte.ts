import { MediaQuery } from "svelte/reactivity";
import type { ThemeMode } from "$lib/api";
import { accentTokens, isHex } from "$lib/color";
import { DEFAULT_ACCENT } from "$lib/constants";
import { settings } from "./settings.svelte";

const CACHE_KEY = "veil:theme";

interface Cached {
  mode: ThemeMode;
  accent: string;
}

function readCache(): Cached {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (raw) {
      const c = JSON.parse(raw) as Partial<Cached>;
      return {
        mode: c.mode === "light" || c.mode === "dark" || c.mode === "system" ? c.mode : "system",
        accent: isHex(c.accent) ? c.accent : DEFAULT_ACCENT,
      };
    }
  } catch {
    /* storage unavailable */
  }
  return { mode: "system", accent: DEFAULT_ACCENT };
}

class ThemeStore {
  #systemDark = new MediaQuery("(prefers-color-scheme: dark)", true);
  #cache = readCache();

  mode = $derived<ThemeMode>(settings.value?.themeMode ?? this.#cache.mode);
  resolved = $derived<"light" | "dark">(
    this.mode === "system" ? (this.#systemDark.current ? "dark" : "light") : this.mode,
  );
  accent = $derived(isHex(settings.value?.accent) ? settings.value.accent : this.#cache.accent);
  tokens = $derived(accentTokens(this.accent, this.resolved));

  /** Writes the resolved theme to <html>. Call inside an $effect so it tracks changes. */
  apply() {
    const root = document.documentElement;
    const t = this.tokens;
    root.dataset.theme = this.resolved;
    root.style.colorScheme = this.resolved;
    root.style.setProperty("--accent", t.accent);
    root.style.setProperty("--accent-solid", t.solid);
    root.style.setProperty("--accent-solid-hover", t.solidHover);
    root.style.setProperty("--accent-solid-end", t.solidEnd);
    root.style.setProperty("--on-accent", t.onAccent);
    root.style.setProperty("--accent-text", t.text);
    try {
      localStorage.setItem(CACHE_KEY, JSON.stringify({ mode: this.mode, accent: this.accent }));
    } catch {
      /* ignore */
    }
  }
}

export const theme = new ThemeStore();
