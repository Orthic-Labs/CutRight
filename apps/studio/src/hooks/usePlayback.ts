import { useEffect, useRef, useState } from "react";
import { findWord } from "../word-lock";
import type { Word } from "../word-lock";
import type { Mode, Variant } from "../types";

// Owns the transport (playhead/playing/video refs) and the word-locked seek
// helpers. Moved out of main.tsx's `App()` per REV2 §14.4
// `hooks/usePlayback.ts` — pure move, no behavior change.
export function usePlayback({
  mode,
  variant,
  words,
  activeVariant,
}: {
  mode: Mode;
  variant: string;
  words: Record<string, Word[]>;
  activeVariant: Variant | undefined;
}) {
  const [playhead, setPlayhead] = useState(0);
  const [playing, setPlaying] = useState(false);
  const videoRefs = useRef<Record<string, HTMLVideoElement | null>>({});
  const cursor = findWord(words[variant] ?? [], playhead);

  useEffect(() => {
    let id = 0;
    const tick = () => {
      const video = videoRefs.current[mode === "compare" ? variant : "source"];
      if (video && !video.paused) setPlayhead(video.currentTime * 1000);
      id = requestAnimationFrame(tick);
    };
    if (playing) id = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(id);
  }, [playing, variant, mode]);

  function seek(ms: number, key = mode === "compare" ? variant : "source") {
    setPlayhead(Math.max(0, ms));
    const video = videoRefs.current[key];
    if (video) video.currentTime = Math.max(0, ms / 1000);
  }
  function activeVideo() {
    return videoRefs.current[mode === "compare" ? variant : "source"];
  }
  function togglePlayback() {
    const video = activeVideo();
    if (!video) return;
    if (video.paused) video.play().catch(() => setPlaying(false));
    else video.pause();
  }
  function pause() {
    activeVideo()?.pause();
    setPlaying(false);
  }
  function playOrIncreaseRate() {
    const video = activeVideo();
    if (!video) return;
    if (video.paused) {
      video.playbackRate = 1;
      video.play().catch(() => setPlaying(false));
      return;
    }
    video.playbackRate = video.playbackRate >= 4 ? 1 : video.playbackRate * 2;
  }
  function frameDuration() {
    return 1000 / Math.max(1, activeVariant?.fps ?? 30);
  }
  function seekWord(direction: number) {
    const list = words[variant] ?? [];
    const current = cursor.word ? list.indexOf(cursor.word) : -1;
    const target =
      list[Math.max(0, Math.min(list.length - 1, current + direction))];
    if (target) seek(target.start_ms);
  }
  function seekSegment(direction: number) {
    const segments = activeVariant?.cut_plan?.segments ?? [];
    const current = segments.findIndex(
      (segment) =>
        playhead >= (segment.output_start_ms ?? 0) &&
        playhead < (segment.output_end_ms ?? Infinity),
    );
    const index = Math.max(0, Math.min(segments.length - 1, current + direction));
    const target = segments[index];
    if (target) seek(target.output_start_ms ?? 0);
  }

  return {
    playhead,
    setPlayhead,
    playing,
    setPlaying,
    videoRefs,
    cursor,
    seek,
    activeVideo,
    togglePlayback,
    pause,
    playOrIncreaseRate,
    frameDuration,
    seekWord,
    seekSegment,
  };
}
