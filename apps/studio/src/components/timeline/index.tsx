// apps/studio/src/components/timeline/index.tsx — CR-V2-B6-012.
import type { ReactNode } from "react";
export function Track(props: { track_id: string; kind: "video"|"audio"|"overlay"|"caption"; children?: ReactNode }) {
  return <section data-track-id={props.track_id} data-kind={props.kind} className="timeline-track">{props.children}</section>;
}
export function Clip(props: { clip_id: string; start_ms: number; duration_ms: number }) {
  return <article data-clip-id={props.clip_id} className="timeline-clip">{props.clip_id} {props.start_ms}ms {props.duration_ms}ms</article>;
}
