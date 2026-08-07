// apps/studio/src/JobCenter.test.tsx — CR-V2-B6-023.
import { describe, it, expect } from "vitest";
import { JobCenter } from "./components/JobCenter";
import { useJobs } from "./hooks/useJobs";
import { renderHook } from "./test-utils";
describe("JobCenter", () => {
  it("JobCenter renders", () => { expect(JobCenter({ jobs: [] })).toBeTruthy(); });
  it("useJobs returns jobs", async () => {
    const hook = await renderHook(useJobs);
    expect(hook.result.current.jobs).toEqual([]);
    await hook.unmount();
  });
});
