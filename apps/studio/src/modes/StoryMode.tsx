// apps/studio/src/modes/StoryMode.tsx — CR-V2-B6-009 Lane A.
import type { ReactNode } from "react";
export interface StoryBeatView {
  beat_id: string;
  label: "hook" | "setup" | "development" | "payoff" | "cta";
  selected_take_id: string;
  confidence: number;
}
export function StoryMode(props: { beats: readonly StoryBeatView[]; children?: ReactNode }) {
  return (
    <main className="story-mode" aria-label="Story">
      <h1>Story</h1>
      <ol>
        {props.beats.map((b) => (
          <li key={b.beat_id}><code>{b.beat_id}</code> · {b.label} · conf {b.confidence.toFixed(2)}</li>
        ))}
      </ol>
      {props.children}
    </main>
  );
}
