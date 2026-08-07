// apps/studio/src/modes/BeatsMode.tsx — CR-V2-B6-009 Lane A.
import type { ReactNode } from "react";
export function BeatsMode(props: { project_id: string; children?: ReactNode }) {
  return (
    <main className="beats-mode" aria-label="Beats">
      <h1>Beats</h1>
      <p>Take swap, preserve/drop, reorder, escalation — all through registered actions.</p>
      {props.children}
    </main>
  );
}
