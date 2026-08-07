// apps/studio/src/components/RunDigest.tsx — CR-V2-B6-010.
import type { ReactNode } from "react";
export interface RunDigestProps { ready: number; review: number; failed: number; children?: ReactNode }
export function RunDigest(props: RunDigestProps) {
  return <aside className="run-digest" aria-label="Run digest"><span>Ready {props.ready}</span><span>Review {props.review}</span><span>Failed {props.failed}</span>{props.children}</aside>;
}
