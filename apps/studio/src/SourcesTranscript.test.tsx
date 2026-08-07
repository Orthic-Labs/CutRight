// apps/studio/src/SourcesTranscript.test.tsx
// CR-V2-B6-008 — Lane A. Smoke test that Sources/Transcript render and that
// relink mismatch fails visibly.
import { describe, it, expect } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { SourcesModeV2 } from "./modes/SourcesModeV2";
import { relinkMatchesHash } from "./components/SourceInspector";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("SourcesTranscript", () => {
  it("rejects relink when candidate hash differs", () => {
    expect(relinkMatchesHash({ source_id: "s1", display_name: "x", path: "/tmp/x", blake3: "abc", probe: { duration_ms: 1, width: 1, height: 1, fps: 30, is_hdr: false }, tracks: [], scenes: [] }, "def")).toBe(false);
  });
  it("accepts relink when candidate hash matches", () => {
    expect(relinkMatchesHash({ source_id: "s1", display_name: "x", path: "/tmp/x", blake3: "abc", probe: { duration_ms: 1, width: 1, height: 1, fps: 30, is_hdr: false }, tracks: [], scenes: [] }, "abc")).toBe(true);
  });
  it("SourcesModeV2 renders a header", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    await act(async () => createRoot(host).render(<SourcesModeV2 sources={[]} />));
    expect(host.querySelector("h1")?.textContent).toMatch(/sources/i);
  });
});
