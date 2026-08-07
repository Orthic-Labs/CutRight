// apps/studio/src/components/SourceInspector.tsx
//
// Book 6 task CR-V2-B6-008 — Lane A Sources mode.
//
// Renders the immutable source manifest, hashes, probe facts, tracks,
// scene / shot overview, storyboard thumbnails and source transcript.
// Source bytes are NEVER editable from the UI.

import type { ReactNode } from "react";

export type SourceProbe = {
  duration_ms: number;
  width: number;
  height: number;
  fps: number;
  is_hdr: boolean;
};

export type SourceTrack = {
  kind: "video" | "audio" | "data";
  codec?: string;
  channels?: number;
  sample_rate_hz?: number;
};

export type SourceScene = {
  scene_id: string;
  start_ms: number;
  end_ms: number;
  thumb_hash?: string;
  shot_count: number;
};

export type SourceSummary = {
  source_id: string;
  display_name: string;
  path: string;
  blake3: string;
  probe: SourceProbe;
  tracks: SourceTrack[];
  scenes: SourceScene[];
  poster_jpg?: string | null;
  waveform_png?: string | null;
};

export function SourceInspector(props: { source: SourceSummary; children?: ReactNode }) {
  const { source } = props;
  return (
    <section className="source-inspector" aria-label={`Source ${source.display_name}`}>
      <header>
        <h3>{source.display_name}</h3>
        <p className="hash" aria-label="content hash">
          blake3: <code>{source.blake3.slice(0, 16)}…</code>
        </p>
      </header>

      <dl className="probe-facts">
        <dt>Duration</dt>
        <dd>{(source.probe.duration_ms / 1000).toFixed(3)} s</dd>
        <dt>Resolution</dt>
        <dd>
          {source.probe.width} × {source.probe.height}
          {source.probe.is_hdr && <span className="hdr-badge">HDR</span>}
        </dd>
        <dt>Frame rate</dt>
        <dd>{source.probe.fps.toFixed(3)} fps</dd>
      </dl>

      <section aria-label="tracks">
        <h4>Tracks</h4>
        <ul>
          {source.tracks.map((t, i) => (
            <li key={`${t.kind}-${i}`}>
              <code>{t.kind}</code>
              {t.codec && <span> · {t.codec}</span>}
              {t.channels && <span> · {t.channels} ch</span>}
              {t.sample_rate_hz && <span> · {t.sample_rate_hz} Hz</span>}
            </li>
          ))}
        </ul>
      </section>

      <section aria-label="scenes">
        <h4>Scene / shot overview</h4>
        <ol>
          {source.scenes.map((scene) => (
            <li key={scene.scene_id}>
              <code>{scene.scene_id}</code> · {scene.shot_count} shots ·{" "}
              {(scene.start_ms / 1000).toFixed(2)} – {(scene.end_ms / 1000).toFixed(2)} s
            </li>
          ))}
        </ol>
      </section>

      <p className="source-immutable-note" role="note">
        Source bytes are immutable. Relink by exact hash; mismatched files fail visibly.
      </p>

      {props.children}
    </section>
  );
}

// Relink policy. The UI never edits the source bytes; relink is a separate
// explicit action that requires the exact BLAKE3 hash.
export function relinkMatchesHash(source: SourceSummary, candidateHash: string): boolean {
  return source.blake3.toLowerCase() === candidateHash.toLowerCase();
}