import { tc } from "../lib/api";
import type { CutMarker } from "../cut-markers";

export function Transport({
  playing,
  onPlaying,
  playhead,
  duration,
  onSeek,
  videoKey,
  markers = [],
}: {
  playing: boolean;
  onPlaying: (x: boolean) => void;
  playhead: number;
  duration: number;
  onSeek: (x: number) => void;
  videoKey?: string;
  markers?: CutMarker[];
}) {
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
