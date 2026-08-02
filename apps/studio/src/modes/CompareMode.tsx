import type React from "react";
import { asset } from "../lib/api";
import { BenchSwitch } from "../components/BenchSwitch";
import { SegmentStrip } from "../components/SegmentStrip";
import { Transport } from "../components/Transport";
import type { CutMarker } from "../cut-markers";
import type { DecisionReason } from "../contracts/review";
import type { Word } from "../word-lock";
import type { Variant } from "../types";

// Renders the COMPARE viewer — the Word-Locked Bench (redesign spec Phase
// 1). Named `CompareMode` (was `Compare` in main.tsx) per REV2 §14.4's
// `modes/CompareMode.tsx` — pure move, then extended with the bench switch.
export function CompareMode({
  variants,
  variant,
  words,
  cursor,
  videoRefs,
  playing,
  onPlaying,
  onSwap,
  onSeek,
  bench,
  delta,
  markers,
  flagging,
  lastFlag,
  onFlag,
}: {
  variants: Variant[];
  variant: string;
  words: Record<string, Word[]>;
  cursor?: Word;
  videoRefs: React.MutableRefObject<Record<string, HTMLVideoElement | null>>;
  playing: boolean;
  onPlaying: (value: boolean) => void;
  onSwap: () => void;
  onSeek: (n: number) => void;
  bench: boolean;
  delta: number | null;
  markers: CutMarker[];
  flagging: boolean;
  lastFlag: { segmentId: string; reason: DecisionReason } | null;
  onFlag: () => void;
}) {
  const active = variants.find((item) => item.id === variant)!;
  return (
    <>
      <div className="compare-head">
        <span className="bench-tag">word-locked</span>
        <BenchSwitch
          variants={variants}
          variant={variant}
          bench={bench}
          delta={delta}
          onSwap={onSwap}
        />
        {lastFlag && (
          <span className="flag-badge" title="latest segment flag">
            ⚑ {lastFlag.segmentId} · {lastFlag.reason}
          </span>
        )}
        <button
          className={`flag-segment ${flagging ? "arming" : ""}`}
          title="Flag the segment at the playhead"
          onClick={onFlag}
        >
          ⚑ Flag segment
        </button>
      </div>
      <div className="video-frame compare-videos">
        {variants.map((item) => (
          <video
            key={item.id}
            ref={(node) => {
              videoRefs.current[item.id] = node;
            }}
            src={asset(item.mp4)}
            muted={item.id !== variant}
            preload="metadata"
            className={item.id === variant ? "visible" : "inactive"}
            onPlay={() => onPlaying(true)}
            onPause={() => onPlaying(false)}
          />
        ))}
        <div className="video-placeholder">
          {variant.toUpperCase()} ROUGH CUT
        </div>
      </div>
      <SegmentStrip
        segments={active.cut_plan?.segments ?? []}
        duration={active.duration_ms ?? 0}
        onSeek={onSeek}
      />
      <Transport
        playing={playing}
        onPlaying={onPlaying}
        playhead={cursor?.start_ms ?? 0}
        duration={active.duration_ms ?? 0}
        onSeek={onSeek}
        markers={markers}
      />
    </>
  );
}
