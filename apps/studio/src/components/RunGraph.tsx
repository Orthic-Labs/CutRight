// apps/studio/src/components/RunGraph.tsx — CR-V2-B6-010.
import type { ReactNode } from "react";
export interface RunGraphProps { stages: readonly { stage_id: string; status: string }[]; children?: ReactNode }
export function RunGraph(props: RunGraphProps) {
  return <svg className="run-graph" aria-label="Run DAG">{props.stages.map((s) => <g key={s.stage_id}><text>{s.stage_id} {s.status}</text></g>)}</svg>;
}
