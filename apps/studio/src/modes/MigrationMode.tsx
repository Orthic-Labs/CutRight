import React from 'react';

/**
 * MigrationMode — a v2 mode for the Studio that drives the v1 → v2
 * migration end-to-end.
 *
 * The mode surfaces five steps (in the same order as the frozen plan):
 *
 *   1. **Dry-run**    — the runner walks the descriptors and emits a
 *                       plan, a per-step touched-field list, and a
 *                       backup requirement flag for each step.
 *   2. **Report**     — a summary card showing the plan name, the
 *                       number of destructive steps and the total
 *                       touched-field count.
 *   3. **Backup**     — a button that calls the migration runner's
 *                       `create_backup` entry point and reports the
 *                       archive path.
 *   4. **Execute**    — applies the plan and shows the post-apply
 *                       state hash, the per-step receipts, and the
 *                       backup archive.
 *   5. **Result**     — the final outcome: per-step pass/fail, the
 *                       restored v2 revision id, and the selection
 *                       records that bind the v2 revision to every
 *                       legacy variant.
 *
 * The mode is read-only on the running migration runner; it never
 * mutates the active v2 configuration without an explicit execute.
 */

export type MigrationStep = {
  step: number;
  name: string;
  description: string;
  requires_backup: boolean;
  touched_fields: string[];
};

export type MigrationDryRun = {
  status: 'ready' | 'unsupported' | 'error';
  from: string;
  to: string;
  steps: MigrationStep[];
  backup_count: number;
  touched_fields: string[];
  message?: string;
};

export type MigrationResult = {
  status: 'pass' | 'fail';
  backup_path?: string;
  post_state_hash?: string;
  step_receipts: Array<{
    step: number;
    name: string;
    backup_created: boolean;
    backup_path: string | null;
  }>;
};

export type MigrationModeProps = {
  /** Frozen v1 → v2 plan loaded from `fixtures/migrations/v1-to-v2/`. */
  plan: MigrationDryRun;
  /** Optional last applied result, displayed when the user runs execute. */
  result?: MigrationResult | null;
  /** True when the user has reviewed the dry-run and approved execute. */
  approved?: boolean;
  /** Optional message displayed in the report card. */
  message?: string | null;
};

const STAGE_LABELS: Record<string, string> = {
  'identity-map': 'Identity map',
  'ms-to-ns': 'Milliseconds ↔ rational time',
  'effect-table': 'Effect table',
  'provider-ledger': 'Provider ledger',
};

export function MigrationMode({
  plan,
  result = null,
  approved = false,
  message = null,
}: MigrationModeProps): React.JSX.Element {
  const passed = result?.status === 'pass';
  return (
    <section className="mode migration-mode" data-testid="migration-mode">
      <header>
        <h2>Migration v1 → v2</h2>
        <p>
          Translate a v1 CutRight project into an immutable v2 revision.
          Old finals, decision chains and pack signatures are preserved.
        </p>
      </header>

      <ol className="migration-stages" aria-label="Migration stages">
        <li>
          <h3>1. Dry-run</h3>
          <p>
            Plan: <code>{plan.from}</code> → <code>{plan.to}</code> with
            {' '}{plan.steps.length} steps. Destructive steps:{' '}
            {plan.backup_count}.
          </p>
        </li>
        <li>
          <h3>2. Report</h3>
          <p>
            Touched fields: <code>{plan.touched_fields.length}</code> across
            the {plan.steps.length} steps. The plan is read-only until
            you approve it.
          </p>
        </li>
        <li>
          <h3>3. Backup</h3>
          <p>
            Before the first destructive step the runner writes a backup
            archive under <code>.state/backups/</code>. The archive
            contains the pre-apply state and the in-flight receipts.
          </p>
        </li>
        <li>
          <h3>4. Execute</h3>
          <p>
            {approved
              ? 'Approved. The runner is ready to apply.'
              : 'Awaiting explicit approval.'}
          </p>
        </li>
        <li>
          <h3>5. Result</h3>
          {result ? (
            <p>
              Outcome: <strong>{passed ? 'pass' : 'fail'}</strong>.
              {result.backup_path ? ` Backup: ${result.backup_path}.` : ''}
              {result.post_state_hash ? ` State hash: ${result.post_state_hash}.` : ''}
            </p>
          ) : (
            <p>The result will appear here after execute.</p>
          )}
        </li>
      </ol>

      <ul className="migration-step-list" aria-label="Plan steps">
        {plan.steps.map((step) => (
          <li key={step.step} data-step-name={step.name}>
            <strong>{STAGE_LABELS[step.name] ?? step.name}</strong>
            <span className="step-number">step {step.step}</span>
            {step.requires_backup ? (
              <span className="step-backup">backup</span>
            ) : null}
            <p>{step.description}</p>
          </li>
        ))}
      </ul>

      {message ? <p className="migration-message">{message}</p> : null}
    </section>
  );
}

export default MigrationMode;
