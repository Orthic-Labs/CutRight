import { useRef } from "react";
import { Kbd, usePlatform } from "@rightkit/platform-ui/react";
import { useFocusTrap } from "../hooks/useFocusTrap";

export function Help({ close }: { close: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const platform = usePlatform();
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
        <p>
          <Kbd chord="Mod+1–5" platform={platform} /> modes ·{" "}
          <Kbd chord="Mod+K" platform={platform} /> commands/sources ·{" "}
          <Kbd chord="Mod+R" platform={platform} /> refresh · 1–9 sources ·{" "}
          <Kbd chord="Esc" platform={platform} /> close
        </p>
      </div>
    </div>
  );
}
