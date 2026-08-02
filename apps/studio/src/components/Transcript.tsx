import { useLayoutEffect, useMemo, useRef } from "react";
import type { Word } from "../word-lock";
import { chunkItems, useWindowedChunks } from "../hooks/useWindowedChunks";

// Renders one row of words and reports its own measured height back to the
// windowing hook so later scroll-range math stays accurate. Kept as a
// child component (rather than measuring in the parent) so each row's
// `useLayoutEffect` only fires when that row's own words change.
function WordChunk({
  words,
  cursorId,
  onSeek,
  onMeasure,
}: {
  words: Word[];
  cursorId?: string;
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
        <button
          className={word.id === cursorId ? "current" : ""}
          aria-current={word.id === cursorId ? "true" : undefined}
          key={word.id}
          onClick={() => onSeek(word.start_ms)}
        >
          {word.text}{" "}
        </button>
      ))}
    </div>
  );
}

export function Transcript({
  words,
  cursor,
  onSeek,
  variant,
}: {
  words: Word[];
  cursor?: Word;
  onSeek: (x: number) => void;
  variant: string;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  // Perf fix (REV2 audit): a 5000-word transcript used to mount one <button>
  // per word regardless of scroll position. Chunking + windowing keeps only
  // the chunks near the viewport mounted; short transcripts (QA fixtures,
  // most real ones) fit in a single chunk so this never changes what they
  // render. See hooks/useWindowedChunks.ts.
  const chunks = useMemo(() => chunkItems(words), [words]);
  const { start, end, offsetTop, offsetBottom, recordHeight } =
    useWindowedChunks(listRef, chunks.length);
  const visible = chunks.slice(start, end);

  return (
    <div className="transcript">
      <div className="rail-head">
        <b>TRANSCRIPT</b>
        <span>{variant}</span>
      </div>
      {/* A11y fix (REV2 audit): the current word previously only had a CSS
          class, so screen readers announced nothing as playback advanced.
          This live region announces just the current word, once per
          cursor change (not every playhead tick) — `aria-current` on the
          button itself (below) covers browse-by-word navigation. */}
      <p className="sr-only" aria-live="polite" aria-atomic="true">
        {cursor?.text ?? ""}
      </p>
      <div className="word-list" ref={listRef}>
        {offsetTop > 0 && (
          <div aria-hidden="true" style={{ height: offsetTop }} />
        )}
        {visible.map((chunk, i) => (
          <WordChunk
            key={start + i}
            words={chunk}
            cursorId={cursor?.id}
            onSeek={onSeek}
            onMeasure={(height) => recordHeight(start + i, height)}
          />
        ))}
        {offsetBottom > 0 && (
          <div aria-hidden="true" style={{ height: offsetBottom }} />
        )}
      </div>
      <code>
        {cursor?.source_word_id ?? "preroll"}
        <br />
        {cursor ? `${cursor.start_ms}–${cursor.end_ms}ms` : "No word at cursor"}
      </code>
    </div>
  );
}
