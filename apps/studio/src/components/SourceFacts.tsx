import type { Word } from "../word-lock";
import type { Source } from "../types";

export function SourceFacts({
  source,
  words,
  onSeek,
}: {
  source?: Source;
  words: Word[];
  onSeek: (ms: number) => void;
}) {
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
          <p>
            {words.map((word) => (
              <button key={word.id} onClick={() => onSeek(word.start_ms)}>
                {word.text}{" "}
              </button>
            ))}
          </p>
        ) : (
          <small>Transcript not available</small>
        )}
      </div>
    </div>
  );
}
