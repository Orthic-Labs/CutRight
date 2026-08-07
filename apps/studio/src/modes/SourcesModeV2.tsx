// apps/studio/src/modes/SourcesModeV2.tsx
//
// Book 6 task CR-V2-B6-008 — Lane A Sources mode.
//
// Renders the immutable sources for the current project, exposes
// per-source probe facts, tracks, scene / shot overview, and the
// storyboard thumbnails. Source bytes are NEVER editable from this
// surface; relink is a separate, explicit action that requires the
// exact BLAKE3 hash.

import { useMemo, useState } from "react";
import {
  type SourceSummary,
  SourceInspector,
  relinkMatchesHash,
} from "../components/SourceInspector";

export function SourcesModeV2(props: {
  sources: readonly SourceSummary[];
  onRelink?: (input: { source_id: string; candidate_hash: string }) => void;
  onSeek?: (ms: number) => void;
}) {
  const { sources } = props;
  const [query, setQuery] = useState("");
  const [activeId, setActiveId] = useState<string | null>(sources[0]?.source_id ?? null);
  const [relinkHash, setRelinkHash] = useState("");
  const [relinkError, setRelinkError] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (q.length === 0) return sources;
    return sources.filter(
      (s) =>
        s.display_name.toLowerCase().includes(q) ||
        s.source_id.toLowerCase().includes(q) ||
        s.blake3.toLowerCase().includes(q),
    );
  }, [sources, query]);

  const active = useMemo(
    () => sources.find((s) => s.source_id === activeId) ?? null,
    [sources, activeId],
  );

  const submitRelink = () => {
    if (!active) return;
    if (!relinkMatchesHash(active, relinkHash.trim())) {
      setRelinkError(
        `Relink refused: candidate hash ${relinkHash.trim().slice(0, 12)}… does not match source blake3 ${active.blake3.slice(0, 12)}….`,
      );
      return;
    }
    setRelinkError(null);
    props.onRelink?.({ source_id: active.source_id, candidate_hash: relinkHash.trim() });
  };

  return (
    <main className="sources-mode-v2" aria-label="Sources">
      <header>
        <h1>Sources</h1>
        <p className="note">
          Source bytes are immutable. The UI never edits them; relink requires an exact BLAKE3 hash.
        </p>
        <label>
          Search
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            placeholder="Filter by name, id, or hash"
            aria-label="Filter sources"
          />
        </label>
      </header>

      <section className="source-rail" aria-label="Source list">
        <ol role="list">
          {filtered.map((s) => (
            <li key={s.source_id}>
              <button
                type="button"
                className={s.source_id === active?.source_id ? "active" : undefined}
                onClick={() => setActiveId(s.source_id)}
                aria-current={s.source_id === active?.source_id ? "true" : undefined}
              >
                <strong>{s.display_name}</strong>
                <code>{s.blake3.slice(0, 12)}…</code>
                <span>
                  {(s.probe.duration_ms / 1000).toFixed(1)} s · {s.probe.width}×{s.probe.height}
                </span>
              </button>
            </li>
          ))}
        </ol>
      </section>

      <section className="source-detail" aria-label="Source detail">
        {active ? (
          <>
            <SourceInspector source={active}>
              <fieldset className="relink" aria-label="Relink source">
                <legend>Relink by exact BLAKE3 hash</legend>
                <label>
                  Candidate hash
                  <input
                    type="text"
                    value={relinkHash}
                    onChange={(e) => setRelinkHash(e.currentTarget.value)}
                    placeholder="paste full blake3"
                    spellCheck={false}
                  />
                </label>
                <button
                  type="button"
                  onClick={submitRelink}
                  disabled={relinkHash.trim().length === 0}
                >
                  Relink
                </button>
                {relinkError && (
                  <p role="alert" className="relink-error">
                    {relinkError}
                  </p>
                )}
              </fieldset>
            </SourceInspector>
          </>
        ) : (
          <p>No source selected.</p>
        )}
      </section>
    </main>
  );
}
