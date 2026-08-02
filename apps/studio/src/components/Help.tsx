import { useRef } from "react";
import { useFocusTrap } from "../hooks/useFocusTrap";

export function Help({ close }: { close: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(true, ref);
  return (
    <div
      ref={ref}
      className="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Keyboard shortcuts"
      tabIndex={-1}
    >
      <div>
        <button aria-label="Close shortcuts" onClick={close}>
          ×
        </button>
        <h2>Keyboard shortcuts</h2>
        <p>
          Space play/pause · J/K/L seek, pause, play/rate · A swap variant · ← → words · ⇧← → segments · ,/. frames
        </p>
        <p>⌘1–4 modes · ⌘K commands/sources · ⌘R refresh · 1–9 sources · Esc close</p>
      </div>
    </div>
  );
}
