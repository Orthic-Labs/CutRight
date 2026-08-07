// apps/studio/src/modes/TimelineMode.tsx — CR-V2-B6-012 Lane B.
import type { ReactNode } from "react";
import { Track, Clip } from "../components/timeline";
export function TimelineMode(props: { timeline_id: string; children?: ReactNode }) {
  return (
    <main className="timeline-mode" aria-label="Timeline">
      <h1>Timeline</h1>
      <Track track_id="video" kind="video"><Clip clip_id="c1" start_ms={0} duration_ms={2000} /></Track>
      {props.children}
    </main>
  );
}
