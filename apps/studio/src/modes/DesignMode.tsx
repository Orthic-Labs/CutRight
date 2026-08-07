// apps/studio/src/modes/DesignMode.tsx — CR-V2-B6-013 Lane B.
import type { ReactNode } from "react";
import { DirectionCard, AssetRequestCard } from "../components/design";
export function DesignMode(props: { revision_id: string; children?: ReactNode }) {
  return (
    <main className="design-mode" aria-label="Design">
      <h1>Design</h1>
      <DirectionCard direction_id="d1" label="Editorial" />
      <AssetRequestCard request_id="r1" />
      {props.children}
    </main>
  );
}
