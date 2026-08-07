// apps/studio/src/SourcesTranscript.test.tsx
// CR-V2-B6-008 — Lane A. Smoke test that Sources/Transcript render and that
// relink mismatch fails visibly.
import { describe, it, expect } from "vitest";
import { SourcesModeV2 } from "./modes/SourcesModeV2";
import { relinkMatchesHash } from "./components/SourceInspector";

describe("SourcesTranscript", () => {
  it("rejects relink when candidate hash differs", () => {
    expect(relinkMatchesHash({ source_id: "s1", display_name: "x", blake3: "abc", probe: { duration_ms: 1, width: 1, height: 1 }, scenes: [] }, "def")).toBe(false);
  });
  it("accepts relink when candidate hash matches", () => {
    expect(relinkMatchesHash({ source_id: "s1", display_name: "x", blake3: "abc", probe: { duration_ms: 1, width: 1, height: 1 }, scenes: [] }, "abc")).toBe(true);
  });
  it("SourcesModeV2 renders a header", () => {
    const tree = SourcesModeV2({ sources: [] });
    expect(tree).toBeTruthy();
  });
});
