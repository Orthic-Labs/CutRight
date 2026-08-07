// apps/studio/src/a11y/index.ts — CR-V2-B6-021 Lane C.
export function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return false;
  return window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
}
export function trapFocus(_container: HTMLElement | null) { /* no-op stub */ }
