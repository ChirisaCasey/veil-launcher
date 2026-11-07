import { EVENTS, on, type NotifyEvent } from "$lib/api";
import { errorMessage } from "$lib/format";

export type ToastLevel = NotifyEvent["level"];

export interface Toast {
  id: number;
  level: ToastLevel;
  message: string;
  title?: string;
}

const DURATION: Record<ToastLevel, number> = {
  info: 4000,
  success: 4000,
  warning: 6000,
  error: 7500,
};

class ToastStore {
  items = $state<Toast[]>([]);
  #next = 1;
  #timers = new Map<number, ReturnType<typeof setTimeout>>();

  push(level: ToastLevel, message: string, title?: string): number {
    // Collapse exact duplicates that arrive back-to-back (e.g. backend notify + UI feedback).
    const dup = this.items.find((t) => t.level === level && t.message === message);
    if (dup) {
      this.#arm(dup.id, level);
      return dup.id;
    }
    const id = this.#next++;
    this.items = [...this.items.slice(-4), { id, level, message, title }];
    this.#arm(id, level);
    return id;
  }

  info = (message: string, title?: string) => this.push("info", message, title);
  success = (message: string, title?: string) => this.push("success", message, title);
  warning = (message: string, title?: string) => this.push("warning", message, title);
  error = (e: unknown, title?: string) => this.push("error", errorMessage(e), title);

  dismiss(id: number) {
    clearTimeout(this.#timers.get(id));
    this.#timers.delete(id);
    this.items = this.items.filter((t) => t.id !== id);
  }

  /** Pause auto-dismiss while hovered. */
  hold(id: number) {
    clearTimeout(this.#timers.get(id));
  }

  release(id: number) {
    const t = this.items.find((x) => x.id === id);
    if (t) this.#arm(id, t.level);
  }

  #arm(id: number, level: ToastLevel) {
    clearTimeout(this.#timers.get(id));
    this.#timers.set(
      id,
      setTimeout(() => this.dismiss(id), DURATION[level]),
    );
  }

  listen() {
    return on<NotifyEvent>(EVENTS.notify, (e) => this.push(e.level, e.message));
  }
}

export const toasts = new ToastStore();
