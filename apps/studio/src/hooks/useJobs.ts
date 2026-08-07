// apps/studio/src/hooks/useJobs.ts — CR-V2-B6-023.
import { useEffect, useState } from "react";
export interface JobPageRow { job_id: string; status: string; stage_id: string | null; error: string | null }
export function useJobs(poll_ms: number = 1000) {
  const [jobs, setJobs] = useState<readonly JobPageRow[]>([]);
  useEffect(() => {
    const id = setInterval(() => setJobs((prev) => prev), poll_ms);
    return () => clearInterval(id);
  }, [poll_ms]);
  return { jobs, setJobs, refresh: () => setJobs((prev) => [...prev]) };
}
