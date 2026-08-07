// apps/studio/src/modes/TranscriptMode.tsx
// CR-V2-B6-008 — Lane A Transcript mode.
// Stubs transcript correction surface; corrections create actions/revisions
// and never mutate source provider output.
import type { ReactNode } from "react";

export interface TranscriptSegmentView {
  segment_id: string;
  start_ms: number;
  end_ms: number;
  text: string;
  speaker_id: string | null;
  confidence: number;
  source_word_ids: readonly string[];
}

export function TranscriptMode(props: {
  project_id: string;
  segments: readonly TranscriptSegmentView[];
  onCorrectText?: (input: { segment_id: string; text: string }) => void;
  children?: ReactNode;
}): React.ReactElement {
  return (
    <main className="transcript-mode" aria-label="Transcript">
      <h1>Transcript</h1>
      <p className="note">
        Corrections create a new revision; source provider output is preserved unchanged.
      </p>
      <ol role="list">
        {props.segments.map((s) => (
          <li key={s.segment_id}>
            <code>{s.segment_id}</code> · {(s.start_ms / 1000).toFixed(2)} s ·{" "}
            <span>{s.text}</span>
          </li>
        ))}
      </ol>
      {props.children}
    </main>
  );
}
