import { useState } from "react";

export type AutonomyMode = "reviewed" | "review_light" | "autonomous";

export interface AutonomyMetrics {
  benchmark_pass: boolean;
  user_approval_count: number;
  regression_count: number;
  critic_disagreement: number;
  integrity_failures: number;
  qa_failures: number;
}

export interface AutonomyTransition {
  from: AutonomyMode;
  to: AutonomyMode;
  reason: string;
  audit_id: string;
  at: string;
}

export interface AutonomyState {
  schema_version: "v2";
  autonomy_id: string;
  format: { content_type: string; platform: string; variant: string };
  mode: AutonomyMode;
  compatible_pack_set: string[];
  benchmark_profile: string;
  sample_count: number;
  metrics: AutonomyMetrics;
  demoted: boolean;
  last_user_approval: string | null;
  transition_history: AutonomyTransition[];
}

/**
 * AutonomyPanel — read-only display of an AutonomyState. The user can
 * inspect the threshold counts, the last approval timestamp, and the
 * demotion log. The note field is opt-in.
 */
export function AutonomyPanel({ state }: { state: AutonomyState }) {
  const [showHistory, setShowHistory] = useState(false);
  return (
    <section data-testid="autonomy-panel">
      <header>
        <h2>Autonomy</h2>
        <span data-mode={state.mode}>{state.mode}</span>
        {state.demoted && <span data-testid="demoted-flag">demoted</span>}
      </header>
      <dl>
        <dt>user_approval_count</dt>
        <dd>{state.metrics.user_approval_count}</dd>
        <dt>benchmark_pass</dt>
        <dd>{String(state.metrics.benchmark_pass)}</dd>
        <dt>regression_count</dt>
        <dd>{state.metrics.regression_count}</dd>
        <dt>critic_disagreement</dt>
        <dd>{state.metrics.critic_disagreement}</dd>
        <dt>integrity_failures</dt>
        <dd>{state.metrics.integrity_failures}</dd>
        <dt>qa_failures</dt>
        <dd>{state.metrics.qa_failures}</dd>
        <dt>last_user_approval</dt>
        <dd>{state.last_user_approval ?? "never"}</dd>
      </dl>
      <button type="button" onClick={() => setShowHistory((v) => !v)}>
        {showHistory ? "Hide" : "Show"} transition log
      </button>
      {showHistory && (
        <ol>
          {state.transition_history.map((t, i) => (
            <li key={i}>
              {t.from} → {t.to} ({t.reason}) @ {t.at}
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

export default AutonomyPanel;
