import { useEffect, useRef, useState } from "react";

// Animates a displayed integer from 0 up to `target` once, whenever
// `target` changes (not on mount with its initial value). Backs the
// word-locked bench's delta badge ("3 words cut here") per the redesign
// spec's motion table: "delta badge counts up once". Plain rAF, no new
// dependency — same floor the rest of the app's motion already holds to.
export function useCountUp(target: number, durationMs = 260) {
  const [value, setValue] = useState(target);
  const prevTarget = useRef(target);

  useEffect(() => {
    if (target === prevTarget.current) return;
    prevTarget.current = target;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced || target <= 0) {
      setValue(target);
      return;
    }
    let raf = 0;
    const start = performance.now();
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / durationMs);
      setValue(Math.round(target * t));
      if (t < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target, durationMs]);

  return value;
}
