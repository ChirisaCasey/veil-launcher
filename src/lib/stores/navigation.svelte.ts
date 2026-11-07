import { EVENTS, on } from "$lib/api";

export type View = "home" | "videos" | "detection" | "settings";

export const VIEWS: View[] = ["home", "videos", "detection", "settings"];

/** Route names the tray (or anything else) may send, mapped to views. */
const ALIASES: Record<string, View> = {
  home: "home",
  dashboard: "home",
  videos: "videos",
  library: "videos",
  video: "videos",
  "change-video": "videos",
  changeVideo: "videos",
  detection: "detection",
  detect: "detection",
  settings: "settings",
  preferences: "settings",
  about: "settings",
};

class NavigationStore {
  view = $state<View>("home");
  /** +1 when moving down the sidebar, -1 when moving up; drives the transition direction. */
  direction = $state<1 | -1>(1);

  go(target: View | string) {
    const next = ALIASES[target] ?? ALIASES[target.toLowerCase()];
    if (!next || next === this.view) return;
    this.direction = VIEWS.indexOf(next) >= VIEWS.indexOf(this.view) ? 1 : -1;
    this.view = next;
  }

  listen() {
    return on<string>(EVENTS.navigate, (route) => this.go(String(route)));
  }
}

export const nav = new NavigationStore();
