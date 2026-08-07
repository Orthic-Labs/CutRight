// apps/studio/src/components/AssetPanel.tsx — CR-V2-B6-015.
import type { ReactNode } from "react";
export interface AssetRow { asset_id: string; kind: string; status: string; rights: string; blake3: string; refs: readonly string[] }
export function AssetPanel(props: { assets: readonly AssetRow[]; children?: ReactNode }) {
  return (
    <aside className="asset-panel" aria-label="Assets">
      <h2>Assets</h2>
      <ol>{props.assets.map((a) => <li key={a.asset_id}><code>{a.asset_id}</code> · {a.kind} · {a.status} · rights {a.rights}</li>)}</ol>
      {props.children}
    </aside>
  );
}
