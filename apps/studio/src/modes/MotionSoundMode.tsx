// apps/studio/src/modes/MotionSoundMode.tsx — CR-V2-B6-014 Lane B.
import type { ReactNode } from "react";
import { EffectSlot, AudioGraph } from "../components/motion";
import { TransientMarker } from "../components/audio";
export function MotionSoundMode(props: { timeline_id: string; children?: ReactNode }) {
  return (
    <main className="motion-sound-mode" aria-label="Motion & Sound">
      <h1>Motion & Sound</h1>
      <EffectSlot slot_id="es1" kind="kenburns" />
      <AudioGraph graph_id="ag1" />
      <TransientMarker transient_id="t1" position_ms={1500} />
      {props.children}
    </main>
  );
}
