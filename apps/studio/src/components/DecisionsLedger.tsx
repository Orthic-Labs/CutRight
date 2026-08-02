import type { DecisionRecord, DecisionReplay } from "../contracts/review";

export function DecisionsLedger({
  flagged,
  malformed,
  close,
}: {
  flagged: DecisionRecord[];
  malformed: DecisionReplay["malformed_lines"];
  close: () => void;
}) {
  return (
    <div className="decisions-panel" role="dialog" aria-label="Decision ledger">
      <div className="panel-head">
        <b>DECISION LEDGER</b>
        <button aria-label="Close ledger" onClick={close}>
          ×
        </button>
      </div>
      {flagged.length === 0 && malformed.length === 0 ? (
        <p className="panel-empty">All records current.</p>
      ) : (
        <ul>
          {flagged.map((record) => (
            <li key={record.decision_id}>
              <span className={`status-chip ${record.status}`}>
                {record.status}
              </span>
              <code>
                {record.kind} · {record.subject}
              </code>
              <small>
                {record.verdict} · {record.reason} · {record.ts}
              </small>
            </li>
          ))}
        </ul>
      )}
      {malformed.length > 0 && (
        <p className="malformed">
          {malformed.length} malformed line
          {malformed.length === 1 ? "" : "s"}:{" "}
          {malformed
            .map((line) => `#${line.line_number} (${line.error})`)
            .join(", ")}
        </p>
      )}
    </div>
  );
}
