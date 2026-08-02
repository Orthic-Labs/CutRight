import type { Word } from "../word-lock";

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
  return (
    <div className="transcript">
      <div className="rail-head">
        <b>TRANSCRIPT</b>
        <span>{variant}</span>
      </div>
      <p>
        {words.map((word) => (
          <button
            className={word.id === cursor?.id ? "current" : ""}
            key={word.id}
            onClick={() => onSeek(word.start_ms)}
          >
            {word.text}{" "}
          </button>
        ))}
      </p>
      <code>
        {cursor?.source_word_id ?? "preroll"}
        <br />
        {cursor ? `${cursor.start_ms}–${cursor.end_ms}ms` : "No word at cursor"}
      </code>
    </div>
  );
}
