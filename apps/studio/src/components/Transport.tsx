import { useCallback, useRef } from "react";
import { tc } from "../lib/api";
import { useLiveIndicator } from "../hooks/useLiveIndicator";
import type { CutMarker } from "../cut-markers";

export function Transport({
  playing,
  onPlaying,
  playhead,
  playheadRef,
  duration,
  onSeek,
  videoKey,
  markers = [],
}: {
  playing: boolean;
  onPlaying: (x: boolean) => void;
  playhead: number;
  // Optional: when provided, the scrub input is driven directly from this
  // ref every animation frame (bypassing React state) so it stays visually
  // smooth during continuous playback without re-rendering the tree. Callers
  // that already pass a coarse, word-snapped `playhead` (compare mode) don't
  // need this — their value only changes on real cursor-word transitions.
  playheadRef?: { current: number };
  duration: number;
  onSeek: (x: number) => void;
  videoKey?: string;
  markers?: CutMarker[];
}) {
  const scrubRef = useRef<HTMLInputElement>(null);
  const applyScrub = useCallback(
    (node: HTMLInputElement, value: number) => {
      node.value = String(Math.min(value, duration));
    },
    [duration],
  );
  useLiveIndicator(
    scrubRef,
    playheadRef ?? { current: playhead },
    Boolean(playheadRef) && playing,
    applyScrub,
  );
  return (
    <div className="transport">
      <button
        aria-label={playing ? "Pause" : "Play"}
        onClick={() => {
          const video = document.querySelector<HTMLVideoElement>(
            videoKey === "source"
              ? ".video-frame video"
              : ".compare-videos video.visible",
          );
          if (playing) video?.pause();
          else video?.play().catch(() => onPlaying(false));
          onPlaying(!playing);
        }}
      >
        {playing ? "Ⅱ" : "▶"}
      </button>
      <code>
        {tc(playhead)} / {tc(duration)}
      </code>
      <div className="scrub">
        <input
          ref={scrubRef}
          aria-label="Scrub"
          type="range"
          min="0"
          max={duration}
          value={Math.min(playhead, duration)}
          onChange={(event) => onSeek(Number(event.target.value))}
        />
        {markers.map((marker) => (
          <i
            key={`${marker.ms}-${marker.label}`}
            className="cut-marker"
            title={`${marker.label} · ${tc(marker.ms)}`}
            style={{
              left: `${Math.min(100, (marker.ms / Math.max(1, duration)) * 100)}%`,
            }}
          />
        ))}
      </div>
    </div>
  );
}
