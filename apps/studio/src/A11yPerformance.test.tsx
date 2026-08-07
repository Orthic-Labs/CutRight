// apps/studio/src/A11yPerformance.test.tsx — CR-V2-B6-021.
import { describe, it, expect } from "vitest";
import { prefersReducedMotion, trapFocus } from "./a11y";
import { INITIAL_LOAD_BUDGET_MS, INTERACTION_BUDGET_MS, MEMORY_BUDGET_MB } from "./performance";
describe("A11yPerformance", () => {
  it("prefersReducedMotion returns boolean", () => { expect(typeof prefersReducedMotion()).toBe("boolean"); });
  it("trapFocus is callable", () => { expect(typeof trapFocus).toBe("function"); });
  it("budgets are positive", () => {
    expect(INITIAL_LOAD_BUDGET_MS).toBeGreaterThan(0);
    expect(INTERACTION_BUDGET_MS).toBeGreaterThan(0);
    expect(MEMORY_BUDGET_MB).toBeGreaterThan(0);
  });
});
