import { useLayoutEffect, useMemo, useRef } from "react";
import type { Word } from "../word-lock";
import type { Source } from "../types";
import { chunkItems, useWindowedChunks } from "../hooks/useWindowedChunks";

// See Transcript.tsx's WordChunk — same measure-and-report pattern, no
// current-word tracking needed here (SourceFacts never highlights a
// cursor word).
function WordChunk({
  words,
  onSeek,
  onMeasure,
}: {
  words: Word[];
  onSeek: (ms: number) => void;
  onMeasure: (height: number) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (ref.current) onMeasure(ref.current.getBoundingClientRect().height);
  });
  return (
    <div className="word-chunk" ref={ref}>
      {words.map((word) => (
        <button key={word.id} onClick={() => onSeek(word.start_ms)}>
          {word.text}{" "}
        </button>
      ))}
    </div>
  );
}

export function SourceFacts({
  source,
  words,
  onSeek,
}: {
  source?: Source;
  words: Word[];
  onSeek: (ms: number) => void;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  // Perf fix (REV2 audit): same unbounded per-word DOM cost as Transcript —
  // windowed the same way. See hooks/useWindowedChunks.ts.
  const chunks = useMemo(() => chunkItems(words), [words]);
  const { start, end, offsetTop, offsetBottom, recordHeight } =
    useWindowedChunks(listRef, chunks.length);
  const visible = chunks.slice(start, end);

  return (
    <div className="source-facts">
      <b>SOURCE FACTS</b>
      <p>{source?.display_name}</p>
      <code>
        {source?.source_id}
        <br />
        {source?.path ?? "Media path unavailable"}
      </code>
      <div className="source-transcript">
        <b>TRANSCRIPT</b>
        {source?.transcript ? (
          <div className="word-list" ref={listRef}>
            {offsetTop > 0 && (
              <div aria-hidden="true" style={{ height: offsetTop }} />
            )}
            {visible.map((chunk, i) => (
              <WordChunk
                key={start + i}
                words={chunk}
                onSeek={onSeek}
                onMeasure={(height) => recordHeight(start + i, height)}
              />
            ))}
            {offsetBottom > 0 && (
              <div aria-hidden="true" style={{ height: offsetBottom }} />
            )}
          </div>
        ) : (
          <small>Transcript not available</small>
        )}
      </div>
    </div>
  );
}
