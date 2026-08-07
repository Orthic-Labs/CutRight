// apps/studio/src/RunMode.test.tsx — CR-V2-B6-010.
import { describe, it, expect } from "vitest";
import { RunMode } from "./modes/RunMode";
import { useRun } from "./hooks/useRun";
import { renderHook } from "./test-utils";
describe("RunMode", () => {
  it("renders empty stages", () => { expect(RunMode({ stages: [] })).toBeTruthy(); });
  it("useRun starts idle", async () => {
    const hook = await renderHook(() => useRun(null));
    expect(hook.result.current.status).toBe("idle");
    await hook.unmount();
  });
});
