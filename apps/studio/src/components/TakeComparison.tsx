// apps/studio/src/components/TakeComparison.tsx — CR-V2-B6-009.
import type { ReactNode } from "react";
export interface TakeView { take_id: string; signals: readonly string[] }
export function TakeComparison(props: { selected: TakeView; alternates: readonly TakeView[]; confidence: number; children?: ReactNode }) {
  return (
    <section className="take-comparison" aria-label="Take comparison">
      <header><strong>{props.selected.take_id}</strong> · conf {props.confidence.toFixed(2)}</header>
      <ul>{props.selected.signals.map((s) => <li key={s}>{s}</li>)}</ul>
      <ol>{props.alternates.map((a) => <li key={a.take_id}>{a.take_id}</li>)}</ol>
      {props.children}
    </section>
  );
}
