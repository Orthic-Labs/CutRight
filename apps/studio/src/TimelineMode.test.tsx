// apps/studio/src/TimelineMode.test.tsx — CR-V2-B6-012.
import { describe, it, expect } from "vitest";
import { TimelineMode } from "./modes/TimelineMode";
import { useTimeline } from "./hooks/useTimeline";
describe("TimelineMode", () => {
  it("renders", () => { expect(TimelineMode({ timeline_id: "t1" })).toBeTruthy(); });
  it("useTimeline starts at given revision", () => {
    const t = useTimeline("r0");
    expect(t.revision).toBe("r0");
    expect(t.playhead_ms).toBe(0);
  });
});
