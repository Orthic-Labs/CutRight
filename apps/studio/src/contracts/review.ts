// Review-decision contract shared by the Studio UI and its browser QA mock.
// The Rust backend (src-tauri/src/decision_contract.rs) is authoritative; these
// types mirror its serde shapes exactly so the IPC contract stays in one place.

export type ReviewTarget =
  | { target_kind: "variant"; variant: string }
  | { target_kind: "final"; preset: string }
  | { target_kind: "segment"; variant: string; segment_id: string }
  | { target_kind: "qa_report"; preset?: string | null };

export type DecisionVerdict = "approved" | "rejected" | "acknowledged";

export type DecisionReason =
  | "pacing"
  | "word_edges"
  | "energy"
  | "length"
  | "looks_right"
  | "captions"
  | "loudness"
  | "framing"
  | "color"
  | "audio"
  | "clipped_word"
  | "too_tight"
  | "too_loose"
  | "bad_boundary"
  | "wrong_take"
  | "reviewed"
  | "other";

export type DecisionIntent = {
  schema_version: number;
  client_request_id: string;
  target: ReviewTarget;
  verdict: DecisionVerdict;
  reason: DecisionReason;
  note?: string | null;
  playhead_ms: number;
  word_id?: string | null;
  source_word_id?: string | null;
};

export type RecordStatus =
  | "current"
  | "stale_artifact"
  | "missing_artifact"
  | "superseded";

export type DecisionRecord = {
  decision_id: string;
  schema_version: number;
  client_request_id: string;
  ts: string;
  project_id: string;
  // Studio-owned immutable identity (REV2 §12.7), distinct from project_id
  // (still folder-name-derived). Empty string on decisions written before
  // this field existed.
  project_instance_id?: string;
  kind: string;
  verdict: string;
  reason: string;
  note?: string | null;
  subject: string;
  subject_blake3?: string | null;
  subject_size?: number | null;
  variant?: string | null;
  segment_id?: string | null;
  preset?: string | null;
  word_id?: string | null;
  source_word_id?: string | null;
  playhead_ms: number;
  bench_resolved: boolean;
  bench_report_blake3?: string | null;
  project_revision?: string | null;
  app_version: string;
  status?: RecordStatus;
};

export type DecisionReplay = {
  records: DecisionRecord[];
  malformed_lines: Array<{
    line_number: number;
    content: string;
    error: string;
  }>;
};

export const SCHEMA_VERSION = 1;
export const WORD_ID = /^ow_\d{6}$/;
export const SOURCE_WORD_ID = /^.+:w_\d{6}$/;

// Reason vocabularies by review target. The UI renders only the set valid for
// the active target; the backend validates independently.
export const SEGMENT_REASONS: DecisionReason[] = [
  "clipped_word",
  "too_tight",
  "too_loose",
  "bad_boundary",
  "wrong_take",
  "other",
];
export const REASONS: Record<
  "compare" | "finals" | "segment",
  DecisionReason[]
> = {
  compare: ["pacing", "word_edges", "energy", "length", "other"],
  finals: [
    "looks_right",
    "captions",
    "loudness",
    "framing",
    "color",
    "audio",
    "other",
  ],
  segment: SEGMENT_REASONS,
};

export type ResolvedTarget = {
  kind: string;
  subject: string;
  variant?: string;
  segment_id?: string;
  preset?: string;
};

// Mirrors the backend's resolve_target: the subject path is derived from the
// target, never supplied by the caller.
export function resolveTarget(target: ReviewTarget): ResolvedTarget {
  switch (target.target_kind) {
    case "variant":
      return {
        kind: "variant_verdict",
        subject: `render/rough-cuts/${target.variant}.mp4`,
        variant: target.variant,
      };
    case "final":
      return {
        kind: "final_verdict",
        subject: `render/finals/${target.preset}.mp4`,
        preset: target.preset,
      };
    case "segment":
      return {
        kind: "segment_flag",
        subject: `render/rough-cuts/${target.variant}.mp4`,
        variant: target.variant,
        segment_id: target.segment_id,
      };
    case "qa_report":
      return {
        kind: "qa_ack",
        subject: "qa/report.json",
        preset: target.preset ?? undefined,
      };
  }
}

// Result of the backend's `verify_sources` command: one row per registered
// source with the expected hash from the immutable manifest and the rehashed
// actual bytes on disk.
export type SourceCheck = {
  source_id: string;
  path: string;
  expected_blake3: string;
  actual_blake3: string | null;
  matches: boolean;
  error: string | null;
};

// Result of `relink_source`: the newly stored path, its BLAKE3, and whether
// that hash matches the manifest entry the source was registered with.
export type RelinkResult = {
  source_id: string;
  path: string;
  blake3: string;
  matches: boolean;
};

// Persisted by `select_variant` and returned by `read_variant_selection`:
// the hash-bound rough-cut base that final renders consume.
export type VariantSelection = {
  schema_version: number;
  variant: string;
  rough_cut_path: string;
  rough_cut_blake3: string;
  rough_cut_size: number;
  selected_at: string;
  selected_by: string;
};

export type SegmentFlagInput = {
  variant: string;
  segment_id: string;
  reason: DecisionReason;
  note?: string | null;
  playhead_ms: number;
  client_request_id?: string;
  word_id?: string | null;
  source_word_id?: string | null;
};

// Builds the intent for flagging a segment at the cursor. Segment flags are
// always rejections of a specific segment of one variant; the subject and
// kind are derived from the target, never supplied by the caller.
export function buildSegmentFlagIntent(input: SegmentFlagInput): DecisionIntent {
  if (!SEGMENT_REASONS.includes(input.reason)) {
    throw new Error(
      `invalid segment reason "${input.reason}" (expected one of ${SEGMENT_REASONS.join(", ")})`,
    );
  }
  let note: string | null = null;
  if (input.reason === "other") {
    note = (input.note ?? "").trim();
    if (note.length < 1 || note.length > 200) {
      throw new Error(`"other" requires a note of 1-200 characters`);
    }
  }
  return {
    schema_version: SCHEMA_VERSION,
    client_request_id: input.client_request_id ?? crypto.randomUUID(),
    target: {
      target_kind: "segment",
      variant: input.variant,
      segment_id: input.segment_id,
    },
    verdict: "rejected",
    reason: input.reason,
    note,
    playhead_ms: input.playhead_ms,
    word_id: input.word_id ?? null,
    source_word_id: input.source_word_id ?? null,
  };
}

// Builds the record the browser QA mock persists, mirroring the backend's
// authoritative construction so QA exercises the real record shape.
export function buildMockRecord(
  intent: DecisionIntent,
  projectId: string,
  seq: number,
): DecisionRecord {
  const resolved = resolveTarget(intent.target);
  return {
    decision_id: `d_mock_${seq}`,
    schema_version: SCHEMA_VERSION,
    client_request_id: intent.client_request_id,
    ts: new Date().toISOString(),
    project_id: projectId,
    project_instance_id: "pin_mock",
    kind: resolved.kind,
    verdict: intent.verdict,
    reason: intent.reason,
    note: intent.note ?? null,
    subject: resolved.subject,
    subject_blake3: "blake3:mock",
    subject_size: 1,
    variant: resolved.variant ?? null,
    segment_id: resolved.segment_id ?? null,
    preset: resolved.preset ?? null,
    word_id: intent.word_id ?? null,
    source_word_id: intent.source_word_id ?? null,
    playhead_ms: intent.playhead_ms,
    bench_resolved: false,
    bench_report_blake3: null,
    project_revision: "blake3:mock-revision",
    app_version: "qa-mock",
    status: "current",
  };
}
