import { useEffect, type RefObject } from "react";

// Mutates a DOM node directly from a continuously-updating ref every
// animation frame, bypassing React state so a 60fps value (the video
// playhead) never forces a render. Used for the transport scrub input and
// the source waveform marker so they stay visually smooth during playback
// while the committed `playhead` React state only updates on the coarse
// cadence usePlayback uses for everything else. Only runs while `active`
// (typically "is this the visible, playing transport"), so idle/paused
// screens do zero work.
export function useLiveIndicator<T extends HTMLElement>(
  nodeRef: RefObject<T | null>,
  valueRef: { current: number },
  active: boolean,
  apply: (node: T, value: number) => void,
) {
  useEffect(() => {
    if (!active) return;
    let id = 0;
    const tick = () => {
      const node = nodeRef.current;
      if (node) apply(node, valueRef.current);
      id = requestAnimationFrame(tick);
    };
    id = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(id);
  }, [active, nodeRef, valueRef, apply]);
}
