import { useCallback, useRef } from "react";
import type React from "react";
import { asset, tc } from "../lib/api";
import { Transport } from "../components/Transport";
import { useLiveIndicator } from "../hooks/useLiveIndicator";
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
  playheadRef,
  onSeek,
  markers,
}: {
  source?: Source;
  videoRef: (node: HTMLVideoElement | null) => void;
  playing: boolean;
  onPlaying: (value: boolean) => void;
  playhead: number;
  playheadRef: { current: number };
  onSeek: (value: number) => void;
  markers: CutMarker[];
}) {
  const duration = source?.duration_ms ?? 0;
  const markerRef = useRef<HTMLElement>(null);
  const applyMarker = useCallback(
    (node: HTMLElement, value: number) => {
      node.style.left = `${Math.min(100, (value / (duration || 1)) * 100)}%`;
    },
    [duration],
  );
  useLiveIndicator(markerRef, playheadRef, playing, applyMarker);

  function onWaveformKey(event: React.KeyboardEvent<HTMLDivElement>) {
    const step = Math.max(1, duration / 100);
    const big = Math.max(step, 1000);
    if (event.key === "ArrowRight") {
      event.preventDefault();
      onSeek(Math.min(duration, playhead + (event.shiftKey ? big : step)));
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      onSeek(Math.max(0, playhead - (event.shiftKey ? big : step)));
    } else if (event.key === "Home") {
      event.preventDefault();
      onSeek(0);
    } else if (event.key === "End") {
      event.preventDefault();
      onSeek(duration);
    }
  }

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
        playheadRef={playheadRef}
        duration={duration}
        onSeek={onSeek}
        videoKey="source"
        markers={markers}
      />
      <div
        className="waveform"
        role="slider"
        tabIndex={0}
        aria-label="Source waveform"
        aria-valuemin={0}
        aria-valuemax={duration}
        aria-valuenow={Math.round(playhead)}
        aria-valuetext={tc(playhead)}
        onKeyDown={onWaveformKey}
        onClick={(event) =>
          onSeek(
            (event.nativeEvent.offsetX / event.currentTarget.clientWidth) *
              duration,
          )
        }
      >
        <i
          ref={markerRef}
          style={{
            left: `${Math.min(100, (playhead / (duration || 1)) * 100)}%`,
          }}
        />
      </div>
    </>
  );
}
