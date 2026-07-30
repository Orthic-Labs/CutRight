import { describe, expect, it } from "vitest";
import {
  buildMockRecord,
  buildSegmentFlagIntent,
  REASONS,
  resolveTarget,
  SCHEMA_VERSION,
  SEGMENT_REASONS,
  SOURCE_WORD_ID,
  WORD_ID,
  type DecisionIntent,
} from "./review";

function intent(overrides: Partial<DecisionIntent> = {}): DecisionIntent {
  return {
    schema_version: SCHEMA_VERSION,
    client_request_id: "req-1",
    target: { target_kind: "variant", variant: "natural" },
    verdict: "approved",
    reason: "pacing",
    note: null,
    playhead_ms: 1000,
    word_id: null,
    source_word_id: null,
    ...overrides,
  };
}

describe("resolveTarget", () => {
  it("derives a canonical relative subject per target", () => {
    expect(resolveTarget({ target_kind: "variant", variant: "natural" })).toEqual({
      kind: "variant_verdict",
      subject: "render/rough-cuts/natural.mp4",
      variant: "natural",
    });
    expect(resolveTarget({ target_kind: "final", preset: "youtube" })).toEqual({
      kind: "final_verdict",
      subject: "render/finals/youtube.mp4",
      preset: "youtube",
    });
    expect(
      resolveTarget({ target_kind: "segment", variant: "tight", segment_id: "segment-001" }),
    ).toEqual({
      kind: "segment_flag",
      subject: "render/rough-cuts/tight.mp4",
      variant: "tight",
      segment_id: "segment-001",
    });
    expect(resolveTarget({ target_kind: "qa_report", preset: null })).toEqual({
      kind: "qa_ack",
      subject: "qa/report.json",
      preset: undefined,
    });
  });

  it("never produces an absolute or traversable subject", () => {
    for (const target of [
      { target_kind: "variant", variant: "../../etc" },
      { target_kind: "final", preset: "/abs/path" },
    ] as const) {
      const resolved = resolveTarget(target);
      // The subject is always rooted at a known canonical prefix; a hostile
      // variant/preset string cannot escape it into an arbitrary absolute path.
      expect(
        resolved.subject.startsWith("render/rough-cuts/") ||
          resolved.subject.startsWith("render/finals/"),
      ).toBe(true);
    }
  });
});

describe("buildMockRecord", () => {
  it("mirrors the backend record shape", () => {
    const record = buildMockRecord(intent(), "project-test", 1);
    expect(record.decision_id).toBe("d_mock_1");
    expect(record.schema_version).toBe(SCHEMA_VERSION);
    expect(record.project_id).toBe("project-test");
    expect(record.kind).toBe("variant_verdict");
    expect(record.verdict).toBe("approved");
    expect(record.reason).toBe("pacing");
    expect(record.subject).toBe("render/rough-cuts/natural.mp4");
    expect(record.variant).toBe("natural");
    expect(record.app_version).toBe("qa-mock");
    expect(record.status).toBe("current");
    expect(record.subject_blake3).toBeTruthy();
    expect(record.project_revision).toBeTruthy();
  });

  it("carries the client_request_id so retries stay idempotent", () => {
    const record = buildMockRecord(
      intent({ client_request_id: "stable-id" }),
      "project-test",
      2,
    );
    expect(record.client_request_id).toBe("stable-id");
  });
});

describe("reason vocabularies", () => {
  it("are target-specific, not verdict-specific", () => {
    expect(REASONS.compare).toEqual(["pacing", "word_edges", "energy", "length", "other"]);
    expect(REASONS.finals).toContain("looks_right");
    expect(REASONS.finals).toContain("color");
    expect(REASONS.finals).toContain("audio");
    // Segment-only reasons never appear in a variant/final verdict vocabulary.
    expect(REASONS.compare).not.toContain("clipped_word");
    expect(REASONS.finals).not.toContain("clipped_word");
  });
});

describe("word id guards", () => {
  it("accept the six-digit forms and reject short ones", () => {
    expect(WORD_ID.test("ow_000003")).toBe(true);
    expect(WORD_ID.test("ow_3")).toBe(false);
    expect(WORD_ID.test("tw_000003")).toBe(false);
    expect(SOURCE_WORD_ID.test("source-a:w_000003")).toBe(true);
    expect(SOURCE_WORD_ID.test("source-a:w_3")).toBe(false);
  });
});

describe("buildSegmentFlagIntent", () => {
  const base = {
    variant: "natural",
    segment_id: "segment-002",
    reason: "too_tight" as const,
    playhead_ms: 3120,
    client_request_id: "req-flag-1",
    word_id: "ow_000005",
    source_word_id: "source-a:w_000005",
  };

  it("builds a rejected segment-target intent with a derived subject", () => {
    const flag = buildSegmentFlagIntent(base);
    expect(flag.target).toEqual({
      target_kind: "segment",
      variant: "natural",
      segment_id: "segment-002",
    });
    expect(flag.verdict).toBe("rejected");
    expect(flag.reason).toBe("too_tight");
    expect(flag.schema_version).toBe(SCHEMA_VERSION);
    expect(flag.client_request_id).toBe("req-flag-1");
    expect(flag.playhead_ms).toBe(3120);
    expect(flag.word_id).toBe("ow_000005");
    expect(flag.note).toBeNull();
    // The mock record round-trips the intent into a segment_flag record.
    const record = buildMockRecord(flag, "project-test", 9);
    expect(record.kind).toBe("segment_flag");
    expect(record.subject).toBe("render/rough-cuts/natural.mp4");
    expect(record.segment_id).toBe("segment-002");
    expect(record.verdict).toBe("rejected");
  });

  it("rejects variant/final verdict reasons for a segment flag", () => {
    expect(() =>
      buildSegmentFlagIntent({ ...base, reason: "pacing" }),
    ).toThrow(/invalid segment reason/);
    expect(() =>
      buildSegmentFlagIntent({ ...base, reason: "looks_right" }),
    ).toThrow(/invalid segment reason/);
  });

  it("requires a trimmed 1-200 char note for other and drops it otherwise", () => {
    expect(() =>
      buildSegmentFlagIntent({ ...base, reason: "other", note: "   " }),
    ).toThrow(/note of 1-200 characters/);
    expect(() =>
      buildSegmentFlagIntent({ ...base, reason: "other", note: "x".repeat(201) }),
    ).toThrow(/note of 1-200 characters/);
    const flag = buildSegmentFlagIntent({
      ...base,
      reason: "other",
      note: "  boundary drifts into the next take  ",
    });
    expect(flag.note).toBe("boundary drifts into the next take");
    // Non-other reasons never carry a note, even if one was typed.
    expect(buildSegmentFlagIntent({ ...base, note: "stray" }).note).toBeNull();
  });

  it("exposes exactly the segment reason vocabulary in the UI table", () => {
    expect(SEGMENT_REASONS).toEqual([
      "clipped_word",
      "too_tight",
      "too_loose",
      "bad_boundary",
      "wrong_take",
      "other",
    ]);
    expect(REASONS.segment).toEqual(SEGMENT_REASONS);
  });
});
