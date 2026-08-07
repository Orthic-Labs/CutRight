// apps/studio/src/modes/RunMode.tsx — CR-V2-B6-010 Lane A.
import type { ReactNode } from "react";
export interface RunStageView { stage_id: string; status: "cached"|"running"|"retry"|"review"|"failure"; deps: readonly string[] }
export function RunMode(props: { stages: readonly RunStageView[]; children?: ReactNode }) {
  return (
    <main className="run-mode" aria-label="Run">
      <h1>Run</h1>
      <button type="button">Make versions</button>
      <ol>{props.stages.map((s) => <li key={s.stage_id}><code>{s.stage_id}</code> · {s.status}</li>)}</ol>
      {props.children}
    </main>
  );
}
