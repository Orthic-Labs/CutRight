// apps/studio/src/components/design/index.tsx — CR-V2-B6-013.
import type { ReactNode } from "react";
export function DirectionCard(props: { direction_id: string; label: string; children?: ReactNode }) {
  return <section data-direction-id={props.direction_id}><h3>{props.label}</h3>{props.children}</section>;
}
export function AssetRequestCard(props: { request_id: string; children?: ReactNode }) {
  return <article data-request-id={props.request_id} className="asset-request">{props.children}</article>;
}
