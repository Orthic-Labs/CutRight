// apps/studio/src/CompareFinalsQa.test.tsx — CR-V2-B6-011.
import { describe, it, expect } from "vitest";
import { CompareModeV2 } from "./modes/CompareModeV2";
import { FinalsModeV2 } from "./modes/FinalsModeV2";
import { QaReceiptsMode } from "./modes/QaReceiptsMode";
describe("CompareFinalsQa", () => {
  it("CompareModeV2 renders", () => { expect(CompareModeV2({ variants: [] })).toBeTruthy(); });
  it("FinalsModeV2 renders", () => { expect(FinalsModeV2({ project_id: "p1" })).toBeTruthy(); });
  it("QaReceiptsMode renders", () => { expect(QaReceiptsMode({ receipt_tree_present: true, tampered: false })).toBeTruthy(); });
});
