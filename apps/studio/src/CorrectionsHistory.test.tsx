// apps/studio/src/CorrectionsHistory.test.tsx — CR-V2-B6-016.
import { describe, it, expect } from "vitest";
import { CorrectionBar } from "./components/CorrectionBar";
import { HistoryPanel } from "./components/HistoryPanel";
import { useHistory } from "./hooks/useHistory";
import { renderHook } from "./test-utils";
describe("CorrectionsHistory", () => {
  it("CorrectionBar renders", () => { expect(CorrectionBar({ actions: [] })).toBeTruthy(); });
  it("HistoryPanel renders", () => { expect(HistoryPanel({ entries: [] })).toBeTruthy(); });
  it("useHistory can_undo starts false", async () => {
    const hook = await renderHook(useHistory);
    expect(hook.result.current.can_undo).toBe(false);
    await hook.unmount();
  });
});
