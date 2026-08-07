// apps/studio/src/modes/CompareModeV2.tsx — CR-V2-B6-011.
import type { ReactNode } from "react";
export function CompareModeV2(props: { variants: readonly string[]; children?: ReactNode }) {
  return (
    <main className="compare-mode-v2" aria-label="Compare">
      <h1>Compare</h1>
      <ol>{props.variants.map((v) => <li key={v}><code>{v}</code></li>)}</ol>
      {props.children}
    </main>
  );
}
