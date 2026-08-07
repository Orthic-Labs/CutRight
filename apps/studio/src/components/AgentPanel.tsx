// apps/studio/src/components/AgentPanel.tsx — CR-V2-B6-018.
import type { ReactNode } from "react";
export function AgentPanel(props: { session_id: string; children?: ReactNode }) {
  return (
    <section className="agent-panel" aria-label="Agent">
      <h2>Agent</h2>
      <code>{props.session_id}</code>
      {props.children}
    </section>
  );
}
