// apps/studio/src/JobCenter.test.tsx — CR-V2-B6-023.
import { describe, it, expect } from "vitest";
import { JobCenter } from "./components/JobCenter";
import { useJobs } from "./hooks/useJobs";
describe("JobCenter", () => {
  it("JobCenter renders", () => { expect(JobCenter({ jobs: [] })).toBeTruthy(); });
  it("useJobs returns jobs", () => { expect(useJobs().jobs).toEqual([]); });
});
