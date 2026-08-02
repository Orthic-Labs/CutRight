import { tc } from "../lib/api";
import type { Source } from "../types";

// Stage badges shown per source row, in pipeline order.
const STAGE_KEYS = [
  "ingested",
  "transcribed",
  "analyzed",
  "in_candidates",
  "in_cut",
] as const;

// The left SOURCES rail, including the per-source stage/integrity badge
// row. Extracted out of App.tsx (REV2 audit decomposition) — pure move, no
// behavior change.
export function SourcesRail({
  sources,
  sourceIndex,
  onSelect,
}: {
  sources: Source[];
  sourceIndex: number;
  onSelect: (index: number) => void;
}) {
  return (
    <aside className="ledger">
      <div className="rail-head">
        <b>SOURCES</b>
        <span>{sources.length}</span>
      </div>
      {sources.map((source, i) => (
        <button
          key={source.source_id}
          className={`source-row ${i === sourceIndex ? "selected" : ""} ${source.file_present === false ? "missing" : ""}`}
          aria-selected={i === sourceIndex}
          onClick={() => onSelect(i)}
        >
          <span className="poster">▣</span>
          <span className="source-copy">
            <strong>{source.display_name ?? source.source_id}</strong>
            <small>
              {tc(source.duration_ms)} ·{" "}
              {source.width ? `${source.width}×${source.height}` : "—"}
            </small>
            <i>
              {STAGE_KEYS.map((key) => (
                <em
                  title={key}
                  className={source.stages?.[key] ? "on" : ""}
                  key={key}
                />
              ))}{" "}
              {source.is_hdr && <mark>HDR</mark>}
              {source.integrity && !source.integrity.granted && (
                <mark
                  className="warn"
                  title={source.integrity.error ?? "not granted for playback"}
                >
                  BLOCKED
                </mark>
              )}
              {source.integrity?.granted && !source.integrity.verified && (
                <mark
                  className="warn"
                  title="current bytes do not match the registered hash"
                >
                  UNVERIFIED
                </mark>
              )}
            </i>
          </span>
        </button>
      ))}
      <footer>
        {tc(sources.reduce((sum, item) => sum + (item.duration_ms ?? 0), 0))}
      </footer>
    </aside>
  );
}
