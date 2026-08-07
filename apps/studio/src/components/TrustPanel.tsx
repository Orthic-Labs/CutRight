import { useState } from "react";

export type TrustOverall =
  | "pass"
  | "pass_with_notes"
  | "fail_repairable"
  | "fail_non_repairable";

export interface TrustFailureRecord {
  component: "sources" | "revisions" | "actions" | "jobs" | "renders" | "qa" | "skills" | "packs";
  path: string;
  failure: "hash_mismatch" | "signature_invalid" | "missing" | "incompatible" | "non_repairable";
  repairable: boolean;
}

export interface TrustStatus {
  overall: TrustOverall;
  sources_ok: boolean;
  revisions_ok: boolean;
  actions_ok: boolean;
  jobs_ok: boolean;
  renders_ok: boolean;
  qa_ok: boolean;
  skills_ok: boolean;
  packs_ok: boolean;
  failures: TrustFailureRecord[];
}

export interface TrustComputation {
  status: TrustStatus;
  can_finalize: boolean;
}

/**
 * TrustPanel — read-only display of a project trust status. Never lets a
 * model override the floor.
 */
export function TrustPanel({ trust }: { trust: TrustComputation }) {
  const [showFailures, setShowFailures] = useState(false);
  const failureCount = trust.status.failures.length;
  return (
    <section data-testid="trust-panel">
      <header>
        <h2>Project Trust</h2>
        <span data-overall={trust.status.overall}>{trust.status.overall}</span>
        {!trust.can_finalize && (
          <span data-testid="cannot-finalize">finalization blocked</span>
        )}
      </header>
      <dl>
        <dt>sources</dt>
        <dd>{String(trust.status.sources_ok)}</dd>
        <dt>revisions</dt>
        <dd>{String(trust.status.revisions_ok)}</dd>
        <dt>actions</dt>
        <dd>{String(trust.status.actions_ok)}</dd>
        <dt>jobs</dt>
        <dd>{String(trust.status.jobs_ok)}</dd>
        <dt>renders</dt>
        <dd>{String(trust.status.renders_ok)}</dd>
        <dt>qa</dt>
        <dd>{String(trust.status.qa_ok)}</dd>
        <dt>skills</dt>
        <dd>{String(trust.status.skills_ok)}</dd>
        <dt>packs</dt>
        <dd>{String(trust.status.packs_ok)}</dd>
      </dl>
      <button type="button" onClick={() => setShowFailures((v) => !v)}>
        {showFailures ? "Hide" : "Show"} failures ({failureCount})
      </button>
      {showFailures && (
        <ol>
          {trust.status.failures.map((f, i) => (
            <li key={i} data-repairable={f.repairable}>
              {f.component} :: {f.path} :: {f.failure}
              {f.repairable ? "" : " (non-repairable)"}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

export default TrustPanel;
