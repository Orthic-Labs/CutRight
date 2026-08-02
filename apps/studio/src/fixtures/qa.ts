// Browser-QA fixture data and in-memory ledger state used when the app runs
// with `?qa=1` (no native Tauri backend). Moved out of main.tsx per REV2
// §14.4 — pure move, no behavior change.

import { type Word } from "../word-lock";
import {
  buildMockRecord,
  SCHEMA_VERSION,
  type DecisionRecord,
  type DecisionReplay,
  type VariantSelection,
} from "../contracts/review";
import { DEFAULT_CLOUD_SETTINGS, type CloudSettings, type Snapshot } from "../types";

export const fixtureWords: Record<string, Word[]> = {
  natural: [
    "This",
    "is",
    "the",
    "same",
    "spoken",
    "sentence",
    "with",
    "room",
    "to",
    "breathe",
  ].map((text, i) => ({
    id: `ow_${String(i).padStart(6, "0")}`,
    text,
    start_ms: i * 620,
    end_ms: i * 620 + 510,
    source_word_id: `source-a:w_${String(i).padStart(6, "0")}`,
  })),
  tight: ["This", "is", "the", "same", "spoken", "sentence", "breathe"].map(
    (text, i) => ({
      id: `ow_${String(i).padStart(6, "0")}`,
      text,
      start_ms: i * 490,
      end_ms: i * 490 + 410,
      source_word_id: `source-a:w_${String(i < 6 ? i : 9).padStart(6, "0")}`,
    }),
  ),
};
export const fixture: Snapshot = {
  project_path: "/QA/Studio.video-project",
  generated_at: "2026-07-26T09:14:22Z",
  manifest: { project_id: "qa-demo", title: "Night walk" },
  sources: [
    {
      source_id: "source-a",
      display_name: "Clip 01 — street",
      duration_ms: 252000,
      width: 3840,
      height: 2160,
      is_hdr: true,
      file_present: true,
      // Exercises the SourcesRail real-thumbnail path (redesign spec: "real
      // thumbnails if an evidence frame exists"); source-b below has none,
      // exercising the duration-labeled placeholder-tile fallback path.
      poster_jpg:
        "data:image/svg+xml;utf8," +
        encodeURIComponent(
          '<svg xmlns="http://www.w3.org/2000/svg" width="88" height="56"><rect width="88" height="56" fill="#2a2a2f"/><path d="M30 18l28 10-28 10z" fill="#84848a"/></svg>',
        ),
      transcript: "analysis/transcripts/source-a.json",
      stages: {
        ingested: true,
        transcribed: true,
        analyzed: true,
        in_candidates: true,
        in_cut: true,
      },
    },
    {
      source_id: "source-b",
      display_name: "Clip 02 — door",
      duration_ms: 104000,
      width: 1920,
      height: 1080,
      file_present: true,
      transcript: "analysis/transcripts/source-b.json",
      stages: {
        ingested: true,
        transcribed: true,
        analyzed: true,
        in_candidates: true,
        in_cut: false,
      },
    },
  ],
  stages: {
    ingested: true,
    transcribed: true,
    analyzed: true,
    candidates: true,
    rough_cut: true,
    final_render: true,
    qa: true,
  },
  variants: [
    {
      id: "natural",
      mp4: "/QA/Studio.video-project/render/rough-cuts/natural.mp4",
      duration_ms: 6200,
      fps: 29.97,
      cut_plan: {
        segments: [
          {
            id: "segment-001",
            output_start_ms: 0,
            output_end_ms: 3100,
            source_start_ms: 0,
            source_end_ms: 3100,
          },
          {
            id: "segment-002",
            output_start_ms: 3100,
            output_end_ms: 6200,
            source_start_ms: 4400,
            source_end_ms: 7500,
          },
        ],
      },
    },
    {
      id: "tight",
      mp4: "/QA/Studio.video-project/render/rough-cuts/tight.mp4",
      duration_ms: 4000,
      fps: 29.97,
      cut_plan: {
        segments: [
          { id: "segment-001", output_start_ms: 0, output_end_ms: 2000 },
          { id: "segment-002", output_start_ms: 2000, output_end_ms: 4000 },
        ],
      },
    },
  ],
  finals: [
    {
      preset: "youtube",
      aspect: "16:9",
      duration_ms: 6200,
      width: 1920,
      height: 1080,
    },
    {
      preset: "reels",
      aspect: "9:16",
      duration_ms: 6100,
      width: 1080,
      height: 1920,
    },
  ],
  qa: {
    status: "pass",
    checks: [
      { id: "Container", status: "pass" },
      { id: "Captions", status: "pass" },
      { id: "Duration", status: "pass" },
    ],
  },
  bench: { decision: "unresolved" },
  decisions_path: "/QA/Studio.video-project/feedback/decisions.jsonl",
};

// Seeded history so browser QA exercises the replay states without the native
// backend: one stale artifact record plus one malformed ledger line.
const seedStale: DecisionRecord = {
  ...buildMockRecord(
    {
      schema_version: SCHEMA_VERSION,
      client_request_id: "qa-seed-stale",
      target: { target_kind: "variant", variant: "tight" },
      verdict: "rejected",
      reason: "pacing",
      note: null,
      playhead_ms: 2400,
      word_id: "ow_000004",
      source_word_id: "source-a:w_000004",
    },
    fixture.manifest.project_id ?? "qa-demo",
    0,
  ),
  decision_id: "d_mock_seed",
  ts: "2026-07-25T18:03:11Z",
  status: "stale_artifact",
};
export const memoryDecisions: DecisionRecord[] = [seedStale];
export const memoryMalformed: DecisionReplay["malformed_lines"] = [
  {
    line_number: 4,
    content: '{"decision_id":"d_trunc',
    error: "EOF while parsing a value",
  },
];
export let memorySelection: VariantSelection | null = null;
export function setMemorySelection(value: VariantSelection | null) {
  memorySelection = value;
}
export let memoryCloudSettings: CloudSettings = { ...DEFAULT_CLOUD_SETTINGS };
export function setMemoryCloudSettings(value: CloudSettings) {
  memoryCloudSettings = value;
}
