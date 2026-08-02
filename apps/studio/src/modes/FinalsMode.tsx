import { asset, tc } from "../lib/api";
import type { VariantSelection } from "../contracts/review";
import type { Snapshot } from "../types";

// Renders the FINALS viewer. Named `FinalsMode` (was `Finals` in main.tsx)
// per REV2 §14.4's `modes/FinalsMode.tsx` — pure move.
export function FinalsMode({
  finals,
  selected,
  onSelect,
  selection,
  selecting,
  onUseFinal,
}: {
  finals: Snapshot["finals"];
  selected: string;
  onSelect: (preset: string) => void;
  selection: VariantSelection | null;
  selecting: string | null;
  onUseFinal: (preset: string) => void;
}) {
  return (
    <div className="finals">
      {finals.map((final) => {
        const chosen = selection?.variant === final.preset;
        return (
          <article
            className={`final-card ${final.aspect === "9:16" ? "vertical" : ""} ${final.preset === selected ? "selected" : ""} ${chosen ? "chosen" : ""}`}
            key={final.preset}
          >
            <div className="final-frame">
              {final.mp4 ? (
                <video src={asset(final.mp4)} controls />
              ) : (
                <span>{final.aspect}</span>
              )}
            </div>
            <div className="final-head">
              <b>{final.preset}</b>
              <button
                aria-pressed={final.preset === selected}
                onClick={() => onSelect(final.preset)}
              >
                {final.preset === selected ? "Reviewing" : "Review"}
              </button>
            </div>
            <code>
              {final.width}×{final.height} · {tc(final.duration_ms)}
            </code>
            <button
              className="use-final"
              disabled={selecting === final.preset}
              aria-pressed={chosen}
              onClick={() => onUseFinal(final.preset)}
            >
              {selecting === final.preset
                ? "Binding…"
                : chosen
                  ? "✓ Base for final"
                  : "Use for final"}
            </button>
            {chosen && selection && (
              <code
                className="selection-hash"
                title={`${selection.rough_cut_path} · ${selection.rough_cut_blake3} · ${selection.rough_cut_size} B · ${selection.selected_by}`}
              >
                {selection.rough_cut_blake3.slice(0, 22)}… ·{" "}
                {selection.selected_at.slice(0, 19).replace("T", " ")}
              </code>
            )}
          </article>
        );
      })}
    </div>
  );
}
