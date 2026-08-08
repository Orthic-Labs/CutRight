// apps/studio/src/CompareFinalsQa.test.tsx — CR-V2-B6-011.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, it, expect, vi } from "vitest";
import { CompareModeV2 } from "./modes/CompareModeV2";
import { FinalsModeV2 } from "./modes/FinalsModeV2";
import { QaReceiptsMode } from "./modes/QaReceiptsMode";
import { VariantAudition } from "./components/finish/VariantAudition";
describe("CompareFinalsQa", () => {
  it("CompareModeV2 renders", () => { expect(CompareModeV2({ variants: [] })).toBeTruthy(); });
  it("FinalsModeV2 renders", () => { expect(FinalsModeV2({ project_id: "p1" })).toBeTruthy(); });
  it("QaReceiptsMode renders", () => { expect(QaReceiptsMode({ receipt_tree_present: true, tampered: false })).toBeTruthy(); });
  it("auditions five keyboard-selectable variants and rejects stale cuts", async () => {
    (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean })
      .IS_REACT_ACT_ENVIRONMENT = true;
    const host = document.createElement("div");
    const root = createRoot(host);
    const onCommit = vi.fn();
    const variants = ["a", "b", "c", "d", "e"].map((id) => ({ id, sourceHashes: ["blake3:" + "0".repeat(64)] }));
    await act(async () => root.render(<VariantAudition variants={variants} lockedCutHash="lock" currentCutHash="lock" onCommit={onCommit} />));
    expect(host.querySelectorAll('[role="option"]')).toHaveLength(5);
    const list = host.querySelector('[role="listbox"]') as HTMLElement;
    await act(async () => list.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
    await act(async () => list.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    expect(onCommit).toHaveBeenCalledWith(variants[1], "lock");
    onCommit.mockClear();
    await act(async () => root.render(<VariantAudition variants={variants} lockedCutHash="lock" currentCutHash="changed" onCommit={onCommit} />));
    await act(async () => (host.querySelector("button") as HTMLButtonElement).click());
    expect(onCommit).not.toHaveBeenCalled();
    expect(host.textContent).toContain("Cut changed");
    await act(async () => root.unmount());
  });
});
