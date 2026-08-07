// apps/studio/src/components/TranscriptEditor.tsx
//
// Book 6 task CR-V2-B6-008 — Lane A Transcript mode.
//
// Renders the source transcript with per-word edit affordances and
// speaker label correction. Text correction writes to a canonical
// corrected layer; it never touches the source provider output. Timing
// edits require a separate reviewed timing action.

import { useMemo, useState } from "react";

export type TranscriptWord = {
  word_id: string;
  text: string;
  start_ms: number;
  end_ms: number;
  speaker_id?: string;
  confidence?: number;
};

export type TranscriptSegment = {
  segment_id: string;
  start_ms: number;
  end_ms: number;
  speaker_id?: string;
  words: TranscriptWord[];
};

export type Transcript = {
  schema: "cutright.transcript/v1";
  source_id: string;
  segments: TranscriptSegment[];
  corrected: boolean;
};

export function TranscriptEditor(props: {
  transcript: Transcript;
  onSeek?: (ms: number) => void;
  onCorrectText?: (input: { word_id: string; new_text: string }) => void;
  onCorrectSpeaker?: (input: { segment_id: string; new_speaker_id: string }) => void;
}) {
  const { transcript, onSeek, onCorrectText, onCorrectSpeaker } = props;
  const [query, setQuery] = useState("");
  const filtered = useMemo(() => {
    if (query.trim().length === 0) return transcript.segments;
    const q = query.trim().toLowerCase();
    return transcript.segments.filter((s) =>
      s.words.some((w) => w.text.toLowerCase().includes(q)),
    );
  }, [query, transcript.segments]);

  return (
    <section className="transcript-editor" aria-label="Transcript editor">
      <header>
        <h3>Transcript</h3>
        <p className="note">
          Text edits write to a canonical corrected layer. Timing changes need a separate reviewed action.
        </p>
        <label>
          Search
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            placeholder="Filter words"
            aria-label="Filter words"
          />
        </label>
      </header>

      <ol className="segments" role="list">
        {filtered.map((seg) => (
          <li key={seg.segment_id} className="segment" data-segment-id={seg.segment_id}>
            <header className="segment-head">
              <button
                className="timecode"
                onClick={() => onSeek?.(seg.start_ms)}
                aria-label={`Seek to ${(seg.start_ms / 1000).toFixed(2)} seconds`}
              >
                {(seg.start_ms / 1000).toFixed(2)} s
              </button>
              {seg.speaker_id !== undefined && (
                <span className="speaker-badge" aria-label={`speaker ${seg.speaker_id}`}>
                  {seg.speaker_id}
                </span>
              )}
              <button
                className="speaker-edit"
                onClick={() => {
                  const next = window.prompt("Speaker id", seg.speaker_id ?? "");
                  if (next !== null) onCorrectSpeaker?.({ segment_id: seg.segment_id, new_speaker_id: next });
                }}
                aria-label="Edit speaker label"
              >
                Edit speaker
              </button>
            </header>
            <p className="words">
              {seg.words.map((w) => (
                <button
                  key={w.word_id}
                  className="word"
                  data-word-id={w.word_id}
                  onClick={() => onSeek?.(w.start_ms)}
                  onDoubleClick={() => {
                    const next = window.prompt("Correct word text", w.text);
                    if (next !== null) onCorrectText?.({ word_id: w.word_id, new_text: next });
                  }}
                  aria-label={`Word ${w.text}`}
                >
                  {w.text}
                </button>
              ))}
            </p>
          </li>
        ))}
      </ol>
    </section>
  );
}

// Pure helper used by the editor and the test fixture: produce a
// "corrected" text layer from a base transcript. The corrected layer is
// keyed by word_id; missing keys fall back to the original text.
export function correctedLayer(
  base: Transcript,
  corrections: ReadonlyMap<string, string>,
): Transcript {
  return {
    ...base,
    corrected: true,
    segments: base.segments.map((seg) => ({
      ...seg,
      words: seg.words.map((w) => ({
        ...w,
        text: corrections.get(w.word_id) ?? w.text,
      })),
    })),
  };
}
