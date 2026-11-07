import { prefersReducedMotion } from "svelte/motion";

/** Transition duration that collapses to 0 when the user prefers reduced motion. */
export function ms(duration: number): number {
  return prefersReducedMotion.current ? 0 : duration;
}

export { prefersReducedMotion };
