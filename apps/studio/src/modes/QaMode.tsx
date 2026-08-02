import type { Snapshot } from "../types";

// Renders the QA viewer. Named `QaMode` (was `Qa` in main.tsx) per REV2
// §14.4's `modes/QaMode.tsx` — pure move.
export function QaMode({
  report,
  acknowledged,
  onAcknowledge,
}: {
  report?: Snapshot["qa"];
  acknowledged: boolean;
  onAcknowledge: () => void;
}) {
  return (
    <div className="qa-report">
      <h2>{report?.status === "pass" ? "✓ QA passed" : "QA pending"}</h2>
      {report?.checks?.map((check) => (
        <div key={check.id}>
          <span className={check.status === "pass" ? "good" : "bad"}>
            {check.status === "pass" ? "✓" : "✕"}
          </span>
          <b>{check.id}</b>
          <small>{check.evidence}</small>
        </div>
      ))}
      <button
        className="approve"
        disabled={acknowledged}
        onClick={onAcknowledge}
      >
        {acknowledged ? "✓ QA acknowledged" : "Acknowledge QA report"}
      </button>
    </div>
  );
}
