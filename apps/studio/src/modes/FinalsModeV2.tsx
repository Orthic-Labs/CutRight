// apps/studio/src/modes/FinalsModeV2.tsx — CR-V2-B6-011.
import type { ReactNode } from "react";
export function FinalsModeV2(props: { project_id: string; children?: ReactNode }) {
  return (
    <main className="finals-mode-v2" aria-label="Finals">
      <h1>Finals</h1>
      <p>Selected variant + receipt hash + critic verdict + final hash.</p>
      {props.children}
    </main>
  );
}
