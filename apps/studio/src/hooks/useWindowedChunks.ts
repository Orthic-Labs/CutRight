import { useLayoutEffect, useRef, useState, type RefObject } from "react";

// A word-count-per-chunk transcripts are split into before windowing. Big
// enough that QA/small real transcripts (a few dozen words) always fit in
// one chunk — so the windowing never changes what a short transcript
// renders — while a 5000-word transcript still only mounts a handful of
// chunks at a time.
export const CHUNK_SIZE = 40;

export function chunkItems<T>(items: T[], size = CHUNK_SIZE): T[][] {
  const chunks: T[][] = [];
  for (let i = 0; i < items.length; i += size) chunks.push(items.slice(i, i + size));
  return chunks;
}

const DEFAULT_ESTIMATE_PX = 46;

// Pure range math, kept separate from the DOM-measuring effect below so it
// is unit-testable without mounting anything: given how tall each chunk
// measured so far (or the running estimate for ones that haven't rendered
// yet), which chunk indices intersect the visible scroll window, plus
// `overscan` chunks of slack on each side, and how much blank space to
// reserve above/below them so the scrollbar and scroll position stay
// correct for the chunks that aren't mounted.
export function computeVisibleChunkRange({
  chunkCount,
  scrollTop,
  viewport,
  heights,
  estimate,
  overscan,
}: {
  chunkCount: number;
  scrollTop: number;
  viewport: number;
  heights: ReadonlyArray<number | undefined>;
  estimate: number;
  overscan: number;
}): { start: number; end: number; offsetTop: number; offsetBottom: number } {
  if (chunkCount === 0) return { start: 0, end: 0, offsetTop: 0, offsetBottom: 0 };
  const heightAt = (i: number) => heights[i] ?? estimate;

  let acc = 0;
  let firstVisible = 0;
  for (; firstVisible < chunkCount; firstVisible += 1) {
    const h = heightAt(firstVisible);
    if (acc + h > scrollTop) break;
    acc += h;
  }
  let lastVisible = firstVisible;
  let bottom = acc;
  while (lastVisible < chunkCount && bottom < scrollTop + viewport) {
    bottom += heightAt(lastVisible);
    lastVisible += 1;
  }

  const start = Math.max(0, firstVisible - overscan);
  const end = Math.min(chunkCount, lastVisible + overscan);

  let offsetTop = 0;
  for (let i = 0; i < start; i += 1) offsetTop += heightAt(i);
  let offsetBottom = 0;
  for (let i = end; i < chunkCount; i += 1) offsetBottom += heightAt(i);

  return { start, end, offsetTop, offsetBottom };
}

function averageHeight(heights: ReadonlyArray<number | undefined>): number {
  const known = heights.filter((h): h is number => h != null);
  if (!known.length) return DEFAULT_ESTIMATE_PX;
  return known.reduce((sum, h) => sum + h, 0) / known.length;
}

// Windows an array of pre-chunked items against the real scroll position of
// `scrollRef`'s element, measuring each rendered chunk's real height so the
// scrollbar and jump-to-word behavior stay correct as different chunks
// mount. No virtualization dependency: the transcript/source-facts panes
// are flowed, wrapped inline text rather than a fixed-row list, so an
// off-the-shelf row virtualizer doesn't apply — this is the small
// custom windowing hook the fix calls for.
export function useWindowedChunks(
  scrollRef: RefObject<HTMLElement | null>,
  chunkCount: number,
  overscan = 2,
) {
  const heights = useRef<Array<number | undefined>>([]);
  const [range, setRange] = useState<{
    start: number;
    end: number;
    offsetTop: number;
    offsetBottom: number;
  }>({ start: 0, end: Math.min(chunkCount, 1 + overscan), offsetTop: 0, offsetBottom: 0 });

  useLayoutEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return;
    const recompute = () => {
      setRange(
        computeVisibleChunkRange({
          chunkCount,
          scrollTop: scrollEl.scrollTop,
          viewport: scrollEl.clientHeight,
          heights: heights.current,
          estimate: averageHeight(heights.current),
          overscan,
        }),
      );
    };
    recompute();
    scrollEl.addEventListener("scroll", recompute, { passive: true });
    const observer = new ResizeObserver(recompute);
    observer.observe(scrollEl);
    return () => {
      scrollEl.removeEventListener("scroll", recompute);
      observer.disconnect();
    };
  }, [scrollRef, chunkCount, overscan]);

  function recordHeight(index: number, height: number) {
    if (heights.current[index] === height) return;
    heights.current[index] = height;
  }

  return { ...range, recordHeight };
}
