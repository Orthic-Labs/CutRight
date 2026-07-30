import { describe, expect, it } from "vitest";
import {
  cutMarkers,
  cutMarkersFromGaps,
  cutMarkersFromPlan,
} from "./cut-markers";
import type { Word } from "./word-lock";

const word = (index: number, start: number, end: number): Word => ({
  id: `ow_${String(index).padStart(6, "0")}`,
  text: "word",
  start_ms: start,
  end_ms: end,
});

describe("cutMarkersFromPlan", () => {
  it("marks each internal segment boundary, not the timeline start", () => {
    const markers = cutMarkersFromPlan([
      { id: "segment-001", output_start_ms: 0, output_end_ms: 3100 },
      { id: "segment-002", output_start_ms: 3100, output_end_ms: 6200 },
      { id: "segment-003", output_start_ms: 6200, output_end_ms: 9000 },
    ]);
    expect(markers).toEqual([
      { ms: 3100, label: "cut → segment-002" },
      { ms: 6200, label: "cut → segment-003" },
    ]);
  });

  it("derives a padded id when a segment has none", () => {
    const markers = cutMarkersFromPlan([
      { output_start_ms: 0, output_end_ms: 1000 },
      { output_start_ms: 1000, output_end_ms: 2000 },
    ]);
    expect(markers).toEqual([{ ms: 1000, label: "cut → segment-002" }]);
  });

  it("returns nothing for a single segment", () => {
    expect(
      cutMarkersFromPlan([{ id: "segment-001", output_start_ms: 0, output_end_ms: 5000 }]),
    ).toEqual([]);
  });
});

describe("cutMarkersFromGaps", () => {
  it("marks gaps at or above the threshold at the earlier word's end", () => {
    const markers = cutMarkersFromGaps(
      [word(0, 0, 500), word(1, 1000, 1400), word(2, 1500, 1900)],
      350,
    );
    expect(markers).toEqual([{ ms: 500, label: "gap 500ms" }]);
  });

  it("ignores gaps below the threshold", () => {
    expect(
      cutMarkersFromGaps([word(0, 0, 500), word(1, 610, 1000)], 350),
    ).toEqual([]);
  });
});

describe("cutMarkers", () => {
  it("prefers the cut plan over transcript gaps", () => {
    const markers = cutMarkers(
      [
        { id: "segment-001", output_start_ms: 0, output_end_ms: 3100 },
        { id: "segment-002", output_start_ms: 3100, output_end_ms: 6200 },
      ],
      [word(0, 0, 500), word(1, 5000, 5400)],
    );
    expect(markers).toEqual([{ ms: 3100, label: "cut → segment-002" }]);
  });

  it("falls back to transcript gaps without a cut plan", () => {
    const markers = cutMarkers(undefined, [
      word(0, 0, 500),
      word(1, 5000, 5400),
    ]);
    expect(markers).toEqual([{ ms: 500, label: "gap 4500ms" }]);
  });
});
