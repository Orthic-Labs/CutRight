// apps/studio/src/components/CorrectionBar.tsx — CR-V2-B6-016 Lane B.
import type { ReactNode } from "react";
export interface CorrectionAction { id: string; label: string; needs_confirm: boolean }
export function CorrectionBar(props: { actions: readonly CorrectionAction[]; onInvoke?: (id: string) => void; children?: ReactNode }) {
  return (
    <nav className="correction-bar" aria-label="Corrections">
      <ul>{props.actions.map((a) => <li key={a.id}><button type="button" onClick={() => props.onInvoke?.(a.id)}>{a.label}{a.needs_confirm ? " ⚠" : ""}</button></li>)}</ul>
      {props.children}
    </nav>
  );
}
