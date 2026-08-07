// apps/studio/src/components/JobCenter.tsx — CR-V2-B6-023.
import type { ReactNode } from "react";
export interface JobRow { job_id: string; status: string; stage_id: string | null; error: string | null }
export function JobCenter(props: { jobs: readonly JobRow[]; onCancel?: (id: string) => void; onResume?: (id: string) => void; children?: ReactNode }) {
  return (
    <section className="job-center" aria-label="Job center">
      <h2>Job center</h2>
      <ol>{props.jobs.map((j) => <li key={j.job_id}><code>{j.job_id}</code> · {j.status} · {j.stage_id ?? "-"} · {j.error ?? ""} <button onClick={() => props.onCancel?.(j.job_id)}>cancel</button> <button onClick={() => props.onResume?.(j.job_id)}>resume</button></li>)}</ol>
      {props.children}
    </section>
  );
}
