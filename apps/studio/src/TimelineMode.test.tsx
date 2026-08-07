// apps/studio/src/TimelineMode.test.tsx — CR-V2-B6-012.
import { describe, it, expect } from "vitest";
import { TimelineMode } from "./modes/TimelineMode";
import { useTimeline } from "./hooks/useTimeline";
import { renderHook } from "./test-utils";
describe("TimelineMode", () => {
  it("renders", () => { expect(TimelineMode({ timeline_id: "t1" })).toBeTruthy(); });
  it("useTimeline starts at given revision", async () => {
    const hook = await renderHook(() => useTimeline("r0"));
    expect(hook.result.current.revision).toBe("r0");
    expect(hook.result.current.playhead_ms).toBe(0);
    await hook.unmount();
  });
});
