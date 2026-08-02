import { describe, expect, it } from "vitest";
import { selectLedgerView } from "./decision-selectors";
import type { DecisionRecord } from "./contracts/review";

function record(overrides: Partial<DecisionRecord>): DecisionRecord {
  return {
    decision_id: "d",
    schema_version: 1,
    client_request_id: "c",
    ts: "2026-01-01T00:00:00Z",
    project_id: "p",
    kind: "variant_verdict",
    verdict: "approved",
    reason: "pacing",
    subject: "s",
    playhead_ms: 0,
    bench_resolved: true,
    app_version: "0.1.0",
    ...overrides,
  };
}

describe("selectLedgerView", () => {
  it("finds the most recent variant verdict for the active variant in compare mode", () => {
    const decisions = [
      record({ decision_id: "1", kind: "variant_verdict", variant: "natural" }),
      record({ decision_id: "2", kind: "variant_verdict", variant: "tight" }),
      record({ decision_id: "3", kind: "variant_verdict", variant: "natural" }),
    ];
    const view = selectLedgerView(decisions, "compare", "natural", "");
    expect(view.latest?.decision_id).toBe("3");
  });

  it("finds the most recent final verdict for the active preset in finals mode", () => {
    const decisions = [
      record({ decision_id: "1", kind: "final_verdict", preset: "youtube" }),
      record({ decision_id: "2", kind: "final_verdict", preset: "shorts" }),
    ];
    const view = selectLedgerView(decisions, "finals", "", "shorts");
    expect(view.latest?.decision_id).toBe("2");
  });

  it("returns undefined latest when nothing matches", () => {
    const view = selectLedgerView(
      [record({ kind: "qa_ack" })],
      "compare",
      "natural",
      "",
    );
    expect(view.latest).toBeUndefined();
  });

  it("ignores verdicts in sources/qa mode", () => {
    const decisions = [record({ kind: "variant_verdict", variant: "natural" })];
    expect(selectLedgerView(decisions, "sources", "natural", "").latest).toBeUndefined();
    expect(selectLedgerView(decisions, "qa", "natural", "").latest).toBeUndefined();
  });

  it("collects only non-current records as flagged, in original order", () => {
    const decisions = [
      record({ decision_id: "1", status: "current" }),
      record({ decision_id: "2", status: "stale_artifact" }),
      record({ decision_id: "3", status: "superseded" }),
      record({ decision_id: "4" }), // no status field at all
    ];
    const view = selectLedgerView(decisions, "compare", "natural", "");
    expect(view.flaggedRecords.map((r) => r.decision_id)).toEqual(["2", "3"]);
  });

  it("counts only stale_artifact and missing_artifact toward staleCount", () => {
    const decisions = [
      record({ status: "stale_artifact" }),
      record({ status: "missing_artifact" }),
      record({ status: "superseded" }),
      record({ status: "current" }),
    ];
    expect(selectLedgerView(decisions, "compare", "natural", "").staleCount).toBe(2);
  });

  it("handles an empty ledger", () => {
    expect(selectLedgerView([], "compare", "natural", "")).toEqual({
      latest: undefined,
      flaggedRecords: [],
      staleCount: 0,
    });
  });
});
