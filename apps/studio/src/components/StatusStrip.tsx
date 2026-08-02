import { artifactIssue } from "../types";
import type { Snapshot, Variant } from "../types";

// Bottom status strip: pipeline progress, bench/QA state, artifact-issue
// warnings, and the decisions-ledger toggle/refresh controls. Extracted
// out of App.tsx (REV2 audit decomposition) — pure move, no behavior
// change.
export function StatusStrip({
  stages,
  bench,
  benchProvisional,
  qa,
  qaArtifact,
  benchArtifact,
  cutPlanArtifact,
  ledgerOpen,
  onToggleLedger,
  sessionCount,
  totalDecisions,
  staleCount,
  malformedCount,
  onRefresh,
}: {
  stages: Record<string, boolean>;
  bench: Snapshot["bench"];
  benchProvisional: boolean;
  qa: Snapshot["qa"];
  qaArtifact: Snapshot["qa_artifact"];
  benchArtifact: Snapshot["bench_artifact"];
  cutPlanArtifact: Variant["cut_plan_artifact"];
  ledgerOpen: boolean;
  onToggleLedger: () => void;
  sessionCount: number;
  totalDecisions: number;
  staleCount: number;
  malformedCount: number;
  onRefresh: () => void;
}) {
  return (
    <footer className="status-strip">
      <span>
        pipeline {Object.values(stages).filter(Boolean).length}/
        {Object.keys(stages).length}
      </span>
      <span className={benchProvisional ? "warn" : "good"}>
        ● bench: {bench?.decision ?? "unavailable"}
      </span>
      <span className={qa?.status === "pass" ? "good" : "warn"}>
        ● QA: {qa?.status ?? "pending"}
      </span>
      {artifactIssue(qaArtifact) && (
        <span className="warn" title={artifactIssue(qaArtifact) ?? ""}>
          ⚠ QA report {artifactIssue(qaArtifact)}
        </span>
      )}
      {artifactIssue(benchArtifact) && (
        <span className="warn" title={artifactIssue(benchArtifact) ?? ""}>
          ⚠ bench report {artifactIssue(benchArtifact)}
        </span>
      )}
      {artifactIssue(cutPlanArtifact) && (
        <span className="warn" title={artifactIssue(cutPlanArtifact) ?? ""}>
          ⚠ cut plan {artifactIssue(cutPlanArtifact)}
        </span>
      )}
      <button
        className="strip-toggle"
        aria-expanded={ledgerOpen}
        onClick={onToggleLedger}
      >
        decisions: session {sessionCount} · total {totalDecisions}
        {staleCount > 0 && ` · ${staleCount} stale`}
        {malformedCount > 0 && ` · ${malformedCount} malformed`}
      </button>
      <button onClick={onRefresh}>Refresh</button>
    </footer>
  );
}
