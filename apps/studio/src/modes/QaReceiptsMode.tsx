// apps/studio/src/modes/QaReceiptsMode.tsx — CR-V2-B6-011.
import type { ReactNode } from "react";
export function QaReceiptsMode(props: { receipt_tree_present: boolean; tampered: boolean; children?: ReactNode }) {
  return (
    <main className="qa-receipts-mode" aria-label="QA & Receipts">
      <h1>QA & Receipts</h1>
      <p>Receipt tree present: {String(props.receipt_tree_present)}</p>
      <p>Tampered: {String(props.tampered)}</p>
      {props.children}
    </main>
  );
}
