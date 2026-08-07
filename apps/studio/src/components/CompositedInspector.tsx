// apps/studio/src/components/CompositedInspector.tsx — CR-V2-B6-019.
import type { ReactNode } from "react";
export interface CompositedSample { frame_index: number; position_ms: number; image_ref: string; visible_object_ids: readonly string[] }
export function CompositedInspector(props: { samples: readonly CompositedSample[]; children?: ReactNode }) {
  return (
    <section className="composited-inspector" aria-label="Composited inspection">
      <h2>Composited inspection</h2>
      <ol>{props.samples.map((s) => <li key={s.frame_index}><code>{s.image_ref}</code> · {s.position_ms}ms · {s.visible_object_ids.join(",")}</li>)}</ol>
      {props.children}
    </section>
  );
}
