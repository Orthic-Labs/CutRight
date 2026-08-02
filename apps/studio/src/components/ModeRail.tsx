import { MODE_ICON } from "./icons";
import { MODE_LABEL, MODE_ORDER, type Mode } from "../types";

// Left-anchored mode rail (redesign spec Phase 3): real weight (icon +
// label + keycap chip) rather than the old floating lowercase tab row.
// Replaces the horizontal `.modes` nav that used to live inside the
// viewer; same `available`/disabled-reason semantics, same click handler.
export function ModeRail({
  mode,
  setMode,
  available,
}: {
  mode: Mode;
  setMode: (mode: Mode) => void;
  available: Record<Mode, boolean>;
}) {
  return (
    <nav className="mode-rail" aria-label="Viewer modes">
      {MODE_ORDER.map((item, index) => {
        const Icon = MODE_ICON[item];
        return (
          <button
            key={item}
            data-mode-item={item}
            disabled={!available[item]}
            aria-disabled={!available[item]}
            aria-current={mode === item ? "page" : undefined}
            aria-keyshortcuts={`Meta+${index + 1}`}
            title={
              !available[item]
                ? `No ${item === "compare" ? "rough cuts yet — run videoctl edit render" : `${item} yet`}`
                : undefined
            }
            className={`mode-rail-item ${mode === item ? "active" : ""}`}
            onClick={() => setMode(item)}
          >
            <Icon />
            <span className="mode-rail-label">{MODE_LABEL[item]}</span>
            <span className="mode-rail-key" aria-hidden="true">
              {index + 1}
            </span>
          </button>
        );
      })}
    </nav>
  );
}
