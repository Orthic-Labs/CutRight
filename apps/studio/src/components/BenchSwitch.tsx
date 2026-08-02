import { useCountUp } from "../hooks/useCountUp";
import type { Variant } from "../types";

// The Word-Locked Bench's signature control (redesign spec Phase 1): "A/B
// swap is the largest interactive element after the video — a two-pane
// split handle with the active variant lit and the delta badge at the lock
// point." A literal simultaneous side-by-side dual-video split view would
// change the playback/audio architecture `usePlayback`/`swap()` already
// depend on (single active `<video>`, one audio channel, existing keyboard
// contract) — out of safe scope for this pass. This delivers the spec's
// functional intent instead: a two-pane pill spanning the compare header,
// a sliding lit thumb marking the active variant (the "split handle"), and
// the swap's word-cut delta as a count-up badge right at the handle.
export function BenchSwitch({
  variants,
  variant,
  bench,
  delta,
  onSwap,
}: {
  variants: Variant[];
  variant: string;
  bench: boolean;
  delta: number | null;
  onSwap: () => void;
}) {
  const count = useCountUp(delta ?? 0);
  return (
    <div className="bench-switch">
      <div
        className="bench-switch-track"
        role="radiogroup"
        aria-label="Active variant"
      >
        <span
          className="bench-switch-thumb"
          aria-hidden="true"
          data-pos={variants.findIndex((item) => item.id === variant)}
          style={{ width: `${100 / Math.max(1, variants.length)}%` }}
        />
        {variants.map((item) => (
          <button
            key={item.id}
            role="radio"
            className={`bench-switch-pane ${item.id === variant ? "active" : ""}`}
            aria-checked={item.id === variant}
            onClick={() => item.id !== variant && onSwap()}
          >
            {item.id}
            {bench && (
              <i
                className="bench-switch-warn"
                title="word timestamps unverified"
                aria-hidden="true"
              />
            )}
          </button>
        ))}
      </div>
      <button
        className="bench-swap-btn"
        aria-label="Swap variants"
        title="Swap variants (A)"
        onClick={onSwap}
      >
        ⇄
      </button>
      {delta !== null && (
        <span className="delta-badge" role="status" aria-live="polite">
          {count === 0
            ? "same words at lock point"
            : `${count} word${count === 1 ? "" : "s"} cut here`}
        </span>
      )}
    </div>
  );
}
