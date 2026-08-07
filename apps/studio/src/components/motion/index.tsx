// apps/studio/src/components/motion/index.tsx — CR-V2-B6-014.
import type { ReactNode } from "react";
export function EffectSlot(props: { slot_id: string; kind: string; children?: ReactNode }) {
  return <div data-slot-id={props.slot_id} data-kind={props.kind} className="effect-slot">{props.children ?? props.kind}</div>;
}
export function AudioGraph(props: { graph_id: string }) { return <div data-graph-id={props.graph_id} className="audio-graph" />; }
