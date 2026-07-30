// Cut-marker derivation for the transport bar. Markers come from the active
// variant's cut plan when one exists (segment boundaries are the real cuts);
// otherwise they fall back to large inter-word gaps in the transcript, which
// is where the rough cut would split.

import type { Word } from "./word-lock";

export type CutMarker = { ms: number; label: string };

export type MarkerSegment = {
  id?: string;
  output_start_ms?: number;
  output_end_ms?: number;
};

// One marker per internal segment boundary (the first segment starts at 0,
// which is not a cut).
export function cutMarkersFromPlan(segments: MarkerSegment[]): CutMarker[] {
  return segments.flatMap((segment, index) => {
    if (index === 0 || typeof segment.output_start_ms !== "number") return [];
    const fallback = `segment-${String(index + 1).padStart(3, "0")}`;
    return [
      {
        ms: segment.output_start_ms,
        label: `cut → ${segment.id ?? fallback}`,
      },
    ];
  });
}

// Fallback for sources without a cut plan: a gap of at least `gapMs` between
// two spoken words marks a split point at the end of the earlier word.
export function cutMarkersFromGaps(words: Word[], gapMs = 350): CutMarker[] {
  const markers: CutMarker[] = [];
  for (let i = 0; i < words.length - 1; i += 1) {
    const gap = words[i + 1].start_ms - words[i].end_ms;
    if (gap >= gapMs) markers.push({ ms: words[i].end_ms, label: `gap ${gap}ms` });
  }
  return markers;
}

export function cutMarkers(
  segments: MarkerSegment[] | undefined,
  words: Word[],
  gapMs = 350,
): CutMarker[] {
  return segments?.length
    ? cutMarkersFromPlan(segments)
    : cutMarkersFromGaps(words, gapMs);
}
