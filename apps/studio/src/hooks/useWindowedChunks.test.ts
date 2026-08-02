import { describe, expect, it } from "vitest";
import { chunkItems, computeVisibleChunkRange, CHUNK_SIZE } from "./useWindowedChunks";

describe("chunkItems", () => {
  it("splits into fixed-size groups with a shorter final group", () => {
    const items = Array.from({ length: 95 }, (_, i) => i);
    const chunks = chunkItems(items, 40);
    expect(chunks.map((c) => c.length)).toEqual([40, 40, 15]);
  });

  it("keeps a small list in a single chunk", () => {
    expect(chunkItems([1, 2, 3], CHUNK_SIZE)).toEqual([[1, 2, 3]]);
  });

  it("returns no chunks for an empty list", () => {
    expect(chunkItems([], CHUNK_SIZE)).toEqual([]);
  });
});

describe("computeVisibleChunkRange", () => {
  const heights = [40, 40, 40, 40, 40, 40, 40, 40, 40, 40];

  it("windows to the chunks intersecting the viewport plus overscan", () => {
    const range = computeVisibleChunkRange({
      chunkCount: 10,
      scrollTop: 200, // chunk 5 starts at 200
      viewport: 100, // covers chunks 5,6 fully-ish
      heights,
      estimate: 40,
      overscan: 1,
    });
    // chunks 5,6,7 intersect [200,300); +/-1 overscan widens to 4..9
    expect(range.start).toBe(4);
    expect(range.end).toBe(9);
  });

  it("reserves offsetTop/offsetBottom space for chunks outside the window", () => {
    const range = computeVisibleChunkRange({
      chunkCount: 10,
      scrollTop: 200,
      viewport: 100,
      heights,
      estimate: 40,
      overscan: 1,
    });
    expect(range.offsetTop).toBe(4 * 40);
    expect(range.offsetBottom).toBe((10 - range.end) * 40);
  });

  it("clamps the window to the chunk count at the start", () => {
    const range = computeVisibleChunkRange({
      chunkCount: 10,
      scrollTop: 0,
      viewport: 50,
      heights,
      estimate: 40,
      overscan: 2,
    });
    expect(range.start).toBe(0);
    expect(range.offsetTop).toBe(0);
  });

  it("clamps the window to the chunk count at the end", () => {
    const range = computeVisibleChunkRange({
      chunkCount: 10,
      scrollTop: 380,
      viewport: 100,
      heights,
      estimate: 40,
      overscan: 5,
    });
    expect(range.end).toBe(10);
    expect(range.offsetBottom).toBe(0);
  });

  it("returns an empty range for zero chunks", () => {
    expect(
      computeVisibleChunkRange({
        chunkCount: 0,
        scrollTop: 0,
        viewport: 100,
        heights: [],
        estimate: 40,
        overscan: 2,
      }),
    ).toEqual({ start: 0, end: 0, offsetTop: 0, offsetBottom: 0 });
  });

  it("falls back to the estimate for unmeasured chunks", () => {
    const range = computeVisibleChunkRange({
      chunkCount: 10,
      scrollTop: 200,
      viewport: 100,
      heights: [],
      estimate: 40,
      overscan: 1,
    });
    expect(range.start).toBe(4);
    expect(range.end).toBe(9);
  });
});
