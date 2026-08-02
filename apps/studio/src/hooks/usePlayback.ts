import { useEffect, useMemo, useRef, useState } from "react";
import { findWord } from "../word-lock";
import type { Word } from "../word-lock";
import type { Mode, Variant } from "../types";

const EMPTY_WORDS: Word[] = [];

// Owns the transport (playhead/playing/video refs) and the word-locked seek
// helpers. Moved out of main.tsx's `App()` per REV2 §14.4
// `hooks/usePlayback.ts` — pure move, no behavior change.
//
// Perf note (REV2 audit fix): the RAF loop below used to call
// `setPlayhead` on every tick, re-rendering the whole tree at 60fps. It now
// keeps the exact value in `playheadRef` (read by the scrub bar's own
// direct-DOM update, by keyboard nudges, and by decision timestamps) and
// only commits React state — `playhead` — when something user-visible
// actually changed: the cursor word (compare mode) or a whole-second tick
// (the mm:ss transport readout, which has no finer resolution anyway).
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
  const [playhead, setPlayheadState] = useState(0);
  const [playing, setPlaying] = useState(false);
  const videoRefs = useRef<Record<string, HTMLVideoElement | null>>({});
  const playheadRef = useRef(0);

  function commitPlayhead(ms: number) {
    playheadRef.current = ms;
    setPlayheadState(ms);
  }

  const activeWords = words[variant] ?? EMPTY_WORDS;
  // Memoized on the word list + the committed (coarse) playhead, not on
  // every render — `findWord` is a linear scan over the transcript.
  const cursor = useMemo(
    () => findWord(activeWords, playhead),
    [activeWords, playhead],
  );

  useEffect(() => {
    let id = 0;
    let lastWordId: string | undefined;
    let lastBucket = -1;
    const tick = () => {
      const video = videoRefs.current[mode === "compare" ? variant : "source"];
      if (video && !video.paused) {
        const ms = video.currentTime * 1000;
        playheadRef.current = ms;
        const bucket = Math.floor(ms / 1000);
        const word =
          mode === "compare" ? findWord(words[variant] ?? [], ms).word : undefined;
        const wordChanged = mode === "compare" && word?.id !== lastWordId;
        if (wordChanged || bucket !== lastBucket) {
          lastWordId = word?.id;
          lastBucket = bucket;
          setPlayheadState(ms);
        }
      }
      id = requestAnimationFrame(tick);
    };
    if (playing) id = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(id);
  }, [playing, variant, mode, words]);

  function seek(ms: number, key = mode === "compare" ? variant : "source") {
    const clamped = Math.max(0, ms);
    commitPlayhead(clamped);
    const video = videoRefs.current[key];
    if (video) video.currentTime = clamped / 1000;
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
    const at = playheadRef.current;
    const current = segments.findIndex(
      (segment) =>
        at >= (segment.output_start_ms ?? 0) &&
        at < (segment.output_end_ms ?? Infinity),
    );
    const index = Math.max(0, Math.min(segments.length - 1, current + direction));
    const target = segments[index];
    if (target) seek(target.output_start_ms ?? 0);
  }

  return {
    playhead,
    setPlayhead: commitPlayhead,
    playheadRef,
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
