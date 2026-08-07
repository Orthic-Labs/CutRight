// apps/studio/src/CorrectionsHistory.test.tsx — CR-V2-B6-016.
import { describe, it, expect } from "vitest";
import { CorrectionBar } from "./components/CorrectionBar";
import { HistoryPanel } from "./components/HistoryPanel";
import { useHistory } from "./hooks/useHistory";
describe("CorrectionsHistory", () => {
  it("CorrectionBar renders", () => { expect(CorrectionBar({ actions: [] })).toBeTruthy(); });
  it("HistoryPanel renders", () => { expect(HistoryPanel({ entries: [] })).toBeTruthy(); });
  it("useHistory can_undo starts false", () => { expect(useHistory().can_undo).toBe(false); });
});
