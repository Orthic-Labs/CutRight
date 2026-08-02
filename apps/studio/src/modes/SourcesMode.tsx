import { asset } from "../lib/api";
import { Transport } from "../components/Transport";
import type { CutMarker } from "../cut-markers";
import type { Source } from "../types";

// Renders the SOURCES viewer. Named `SourcesMode` (was `Sources` in
// main.tsx) per REV2 §14.4's `modes/SourcesMode.tsx` — pure move.
export function SourcesMode({
  source,
  videoRef,
  playing,
  onPlaying,
  playhead,
  onSeek,
  markers,
}: {
  source?: Source;
  videoRef: (node: HTMLVideoElement | null) => void;
  playing: boolean;
  onPlaying: (value: boolean) => void;
  playhead: number;
  onSeek: (value: number) => void;
  markers: CutMarker[];
}) {
  if (!source) return <div className="empty-state">No source selected.</div>;
  return (
    <>
      <div className="video-frame">
        {source.path ? (
          <video
            ref={videoRef}
            src={asset(source.path)}
            onPlay={() => onPlaying(true)}
            onPause={() => onPlaying(false)}
            onError={() => onPlaying(false)}
          />
        ) : (
          <div className="video-placeholder">SOURCE PREVIEW</div>
        )}
      </div>
      <Transport
        playing={playing}
        onPlaying={onPlaying}
        playhead={playhead}
        duration={source.duration_ms ?? 0}
        onSeek={onSeek}
        videoKey="source"
        markers={markers}
      />
      <div
        className="waveform"
        role="slider"
        aria-label="Source waveform"
        onClick={(event) =>
          onSeek(
            (event.nativeEvent.offsetX / event.currentTarget.clientWidth) *
              (source.duration_ms ?? 0),
          )
        }
      >
        <i
          style={{
            left: `${Math.min(100, (playhead / (source.duration_ms || 1)) * 100)}%`,
          }}
        />
      </div>
    </>
  );
}
