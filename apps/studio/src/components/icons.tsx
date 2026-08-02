// Minimal inline-SVG glyph set for the mode rail. Deliberately not an icon
// library (MINIMIZE — five 16x16 geometric marks don't earn a dependency):
// plain stroke-based shapes matching the app's "Precision" register —
// straight lines, right angles, no ornament. `aria-hidden` on every glyph;
// the rail button itself carries the accessible name.
import type { ReactElement } from "react";
import type { Mode } from "../types";

const common = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.4,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
};

// sources: a stacked-clips glyph (rail of source rows)
function SourcesIcon() {
  return (
    <svg {...common}>
      <rect x="2" y="2.5" width="12" height="3" rx="0.5" />
      <rect x="2" y="6.5" width="12" height="3" rx="0.5" />
      <rect x="2" y="10.5" width="8" height="3" rx="0.5" />
    </svg>
  );
}

// compare: a split-pane glyph (the bench)
function CompareIcon() {
  return (
    <svg {...common}>
      <rect x="1.5" y="2.5" width="13" height="11" rx="1" />
      <line x1="8" y1="2.5" x2="8" y2="13.5" />
    </svg>
  );
}

// finals: a framed check (approved output)
function FinalsIcon() {
  return (
    <svg {...common}>
      <rect x="2" y="2" width="12" height="12" rx="1.5" />
      <path d="M5 8.2l2 2 4-4.4" />
    </svg>
  );
}

// qa: a checklist glyph
function QaIcon() {
  return (
    <svg {...common}>
      <path d="M3 4h10M3 8h10M3 12h6" />
      <path d="M12 10.5l1 1 2-2.2" />
    </svg>
  );
}

// settings: a gear-adjacent dial (two concentric marks, not a full gear —
// keeps the same 1.4px stroke weight as the other four at 16px)
function SettingsIcon() {
  return (
    <svg {...common}>
      <circle cx="8" cy="8" r="2.6" />
      <path d="M8 1.6v2M8 12.4v2M1.6 8h2M12.4 8h2M3.5 3.5l1.4 1.4M11.1 11.1l1.4 1.4M3.5 12.5l1.4-1.4M11.1 4.9l1.4-1.4" />
    </svg>
  );
}

export const MODE_ICON: Record<Mode, () => ReactElement> = {
  sources: SourcesIcon,
  compare: CompareIcon,
  finals: FinalsIcon,
  qa: QaIcon,
  settings: SettingsIcon,
};
