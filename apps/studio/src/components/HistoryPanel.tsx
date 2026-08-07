// apps/studio/src/components/HistoryPanel.tsx — CR-V2-B6-016.
import type { ReactNode } from "react";
export interface HistoryEntry { entry_id: string; producer: string; revision: string; summary: string; undoable: boolean; redoable: boolean }
export function HistoryPanel(props: { entries: readonly HistoryEntry[]; children?: ReactNode }) {
  return (
    <aside className="history-panel" aria-label="History">
      <h2>History</h2>
      <ol>{props.entries.map((e) => <li key={e.entry_id}><code>{e.entry_id}</code> · {e.producer} · rev {e.revision} · {e.summary} · undo={String(e.undoable)} redo={String(e.redoable)}</li>)}</ol>
      {props.children}
    </aside>
  );
}
