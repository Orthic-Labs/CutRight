import { useEffect, useRef, type RefObject } from "react";

// Elements a keyboard user can land on. Matches the common a11y-pattern
// subset (links with an href, non-disabled form controls, explicit
// tabindex) rather than pulling in a library for this.
const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "textarea:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

// Pure DOM query so the "what can I tab to" logic is unit-testable without
// mounting React or simulating a real dialog lifecycle. None of the three
// dialogs this hook is used in conditionally hide a focusable descendant
// (they conditionally *render* it, which the selector already handles), so
// this doesn't need a visibility check beyond the `:not([disabled])` the
// selector itself encodes.
export function getFocusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(
    container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR),
  );
}

// Cycles Tab/Shift+Tab focus inside `container`, wrapping at the ends so
// focus can never leak to the page behind a modal. Exported standalone so
// it can be unit-tested against a real jsdom container without a React
// tree.
export function trapTabKey(event: KeyboardEvent, container: HTMLElement): void {
  if (event.key !== "Tab") return;
  const focusable = getFocusableElements(container);
  if (!focusable.length) {
    event.preventDefault();
    return;
  }
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  const current = document.activeElement;
  if (event.shiftKey) {
    if (current === first || !container.contains(current)) {
      event.preventDefault();
      last.focus();
    }
  } else if (current === last || !container.contains(current)) {
    event.preventDefault();
    first.focus();
  }
}

// Moves focus into `containerRef` while `active` is true, traps Tab
// cycling inside it, and restores focus to whatever had it beforehand once
// `active` goes false or the component unmounts. Shared by every modal
// dialog (CommandPalette, Help, DecisionsLedger) so none of them silently
// drop a keyboard user into the background page or lose their place when
// the dialog closes.
export function useFocusTrap(
  active: boolean,
  containerRef: RefObject<HTMLElement | null>,
) {
  const openerRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!active) return;
    const container = containerRef.current;
    if (!container) return;
    openerRef.current = document.activeElement as HTMLElement | null;
    const focusable = getFocusableElements(container);
    (focusable[0] ?? container).focus();

    const onKeyDown = (event: KeyboardEvent) => trapTabKey(event, container);
    container.addEventListener("keydown", onKeyDown);
    return () => {
      container.removeEventListener("keydown", onKeyDown);
      openerRef.current?.focus();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, containerRef]);
}
