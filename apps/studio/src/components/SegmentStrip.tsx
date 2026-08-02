import { tc } from "../lib/api";
import type { Segment } from "../types";

export function SegmentStrip({
  segments,
  duration,
  onSeek,
}: {
  segments: Segment[];
  duration: number;
  onSeek: (ms: number) => void;
}) {
  return (
    <div className="segments" aria-label="Edit segments">
      {segments.map((segment, i) => (
        <button
          key={segment.id ?? i}
          title={`${segment.id ?? `segment-${i + 1}`} · ${tc((segment.output_end_ms ?? 0) - (segment.output_start_ms ?? 0))}`}
          style={{
            width: `${(((segment.output_end_ms ?? 0) - (segment.output_start_ms ?? 0)) / Math.max(1, duration)) * 100}%`,
          }}
          onClick={() => onSeek(segment.output_start_ms ?? 0)}
        >
          {segment.id ?? `segment-${i + 1}`}
        </button>
      ))}
    </div>
  );
}
