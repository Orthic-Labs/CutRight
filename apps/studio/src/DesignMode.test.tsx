// apps/studio/src/DesignMode.test.tsx — CR-V2-B6-013.
import { describe, it, expect } from "vitest";
import { DesignMode } from "./modes/DesignMode";
import { useDesign } from "./hooks/useDesign";
describe("DesignMode", () => {
  it("renders", () => { expect(DesignMode({ revision_id: "rev1" })).toBeTruthy(); });
  it("useDesign starts with no direction", () => { expect(useDesign("rev1").accepted_direction).toBeNull(); });
});
