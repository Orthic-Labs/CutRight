// Pure selectors over the decision ledger, extracted out of App() so the
// render body can wrap them in a single `useMemo` instead of running a
// reversed-copy `.find()` plus two separate `.filter()` passes over
// `decisions` on every render (App.tsx items: latest verdict badge, the
// flagged-records list for the ledger panel, and the stale-record count in
// the status strip).
import type { DecisionRecord } from "./contracts/review";
import type { Mode } from "./types";

export type LedgerView = {
  latest?: DecisionRecord;
  flaggedRecords: DecisionRecord[];
  staleCount: number;
};

export function selectLedgerView(
  decisions: DecisionRecord[],
  mode: Mode,
  variant: string,
  finalPreset: string,
): LedgerView {
  let latest: DecisionRecord | undefined;
  for (let i = decisions.length - 1; i >= 0; i -= 1) {
    const decision = decisions[i];
    if (
      (mode === "compare" &&
        decision.kind === "variant_verdict" &&
        decision.variant === variant) ||
      (mode === "finals" &&
        decision.kind === "final_verdict" &&
        decision.preset === finalPreset)
    ) {
      latest = decision;
      break;
    }
  }

  const flaggedRecords: DecisionRecord[] = [];
  let staleCount = 0;
  for (const decision of decisions) {
    if (decision.status && decision.status !== "current") {
      flaggedRecords.push(decision);
      if (
        decision.status === "stale_artifact" ||
        decision.status === "missing_artifact"
      )
        staleCount += 1;
    }
  }

  return { latest, flaggedRecords, staleCount };
}
