// apps/studio/src/components/BeatCard.tsx — CR-V2-B6-009.
import type { ReactNode } from "react";
export interface BeatCardProps { beat_id: string; label: string; confidence: number; children?: ReactNode }
export function BeatCard(props: BeatCardProps) {
  return <article className="beat-card" data-beat-id={props.beat_id}><h3>{props.label}</h3><span>conf {props.confidence.toFixed(2)}</span>{props.children}</article>;
}
