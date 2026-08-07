// apps/studio/src/hooks/useRun.ts — CR-V2-B6-010.
import { useEffect, useState } from "react";
export function useRun(job_id: string | null) {
  const [status, setStatus] = useState<"idle"|"running"|"complete"|"failed">("idle");
  useEffect(() => { if (job_id) setStatus("running"); }, [job_id]);
  return { job_id, status, setStatus };
}
