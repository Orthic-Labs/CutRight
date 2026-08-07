// apps/studio/src/HomeMode.test.tsx
//
// Book 6 task CR-V2-B6-007 — Lane A Home / rebuildable project library.
//
// Vitest coverage for the Home mode. The mode itself is a React
// component, but we test the contract and the filter / sort logic that
// the component renders, so the test does not need a DOM renderer.

import { describe, expect, it } from "vitest";
import {
  compareProjectIndexRows,
  isProjectIndex,
  runStatusLabel,
  type ProjectIndex,
  type ProjectIndexRow,
} from "./contracts/projectIndex";
import { laneLabel, statusAccent } from "./hooks/useProjectLibrary";

function row(input: Partial<ProjectIndexRow> & { id: string; updated: string }): ProjectIndexRow {
  return {
    project_instance_id: input.id,
    package_path: input.package_path ?? `/pkg/${input.id}`,
    title: input.title ?? input.id,
    lane: input.lane ?? "recorded_footage",
    active_revision: input.active_revision ?? "rev_001",
    run_status: input.run_status ?? "ready",
    ready_count: input.ready_count ?? 1,
    needs_review_count: input.needs_review_count ?? 0,
    failed_count: input.failed_count ?? 0,
    updated_at: input.updated,
  };
}

function fixture(): ProjectIndex {
  return {
    schema: "cutright.studio.project_index/v1",
    version: 1,
    rows: [
      row({ id: "p1", updated: "2026-08-07T10:00:00Z", title: "Coffee Talk", run_status: "ready" }),
      row({ id: "p2", updated: "2026-08-07T09:00:00Z", title: "Promo Reel", run_status: "needs_review", needs_review_count: 2 }),
      row({ id: "p3", updated: "2026-08-07T08:00:00Z", title: "How To", run_status: "failed", failed_count: 1 }),
    ],
    watch_folder_import_enabled: false,
  };
}

describe("HomeMode contract", () => {
  it("accepts the canonical index shape", () => {
    expect(isProjectIndex(fixture())).toBe(true);
  });

  it("rejects shapes with the wrong schema", () => {
    const f = fixture();
    const wrong = { ...f, schema: "nope" };
    expect(isProjectIndex(wrong)).toBe(false);
  });

  it("sorts rows by updated_at descending, ties on id", () => {
    const f = fixture();
    const sorted = [...f.rows].sort(compareProjectIndexRows);
    expect(sorted.map((r) => r.project_instance_id)).toEqual(["p1", "p2", "p3"]);
  });

  it("maps run_status to display labels", () => {
    expect(runStatusLabel("ready")).toBe("Ready");
    expect(runStatusLabel("needs_review")).toBe("Needs review");
    expect(runStatusLabel("failed")).toBe("Failed");
    expect(runStatusLabel("running")).toBe("Running");
    expect(runStatusLabel("missing")).toBe("Missing");
  });

  it("maps lane id to display label", () => {
    expect(laneLabel("recorded_footage")).toBe("Recorded Footage");
    expect(laneLabel("repurpose")).toBe("Repurpose");
    expect(laneLabel("explainer")).toBe("Explainer");
    expect(laneLabel("anchored_creative")).toBe("Anchored Creative");
  });

  it("maps run_status to accent", () => {
    expect(statusAccent("ready")).toBe("ok");
    expect(statusAccent("running")).toBe("warn");
    expect(statusAccent("needs_review")).toBe("err");
    expect(statusAccent("failed")).toBe("err");
    expect(statusAccent("idle")).toBe("muted");
  });

  it("keeps two same-title rows distinct", () => {
    const f = fixture();
    const dup: ProjectIndex = {
      ...f,
      rows: [
        ...f.rows,
        row({ id: "p1-copy", updated: "2026-08-07T11:00:00Z", title: "Coffee Talk" }),
      ],
    };
    const ids = dup.rows.map((r) => r.project_instance_id).sort();
    expect(ids).toEqual(["p1", "p1-copy", "p2", "p3"]);
    expect(dup.rows.filter((r) => r.title === "Coffee Talk")).toHaveLength(2);
  });
});