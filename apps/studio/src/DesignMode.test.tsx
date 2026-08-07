// apps/studio/src/DesignMode.test.tsx — CR-V2-B6-013.
import { describe, it, expect } from "vitest";
import { DesignMode } from "./modes/DesignMode";
import { useDesign } from "./hooks/useDesign";
import { renderHook } from "./test-utils";
describe("DesignMode", () => {
  it("renders", () => { expect(DesignMode({ revision_id: "rev1" })).toBeTruthy(); });
  it("useDesign starts with no direction", async () => {
    const hook = await renderHook(() => useDesign("rev1"));
    expect(hook.result.current.accepted_direction).toBeNull();
    await hook.unmount();
  });
});
