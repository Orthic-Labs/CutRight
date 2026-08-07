// apps/studio/src/components/AuditionPanel.tsx — CR-V2-B6-015.
import type { ReactNode } from "react";
export interface Audition { audition_id: string; kind: string; preview_hash: string; blinded: boolean }
export function AuditionPanel(props: { auditions: readonly Audition[]; children?: ReactNode }) {
  return (
    <aside className="audition-panel" aria-label="Auditions">
      <h2>Auditions</h2>
      <ul>{props.auditions.map((a) => <li key={a.audition_id}><code>{a.audition_id}</code> · {a.kind} · blinded {String(a.blinded)} · hash {a.preview_hash.slice(0, 8)}…</li>)}</ul>
      {props.children}
    </aside>
  );
}
