import type { DecisionReason } from "../contracts/review";

export function Reason({
  kind,
  title,
  reasons,
  note,
  setNote,
  commit,
  onCancel,
}: {
  kind: "approved" | "rejected";
  title?: string;
  reasons: DecisionReason[];
  note: string;
  setNote: (x: string) => void;
  commit: (x: DecisionReason) => void;
  onCancel?: () => void;
}) {
  return (
    <div className="reason-row">
      <span>
        {title ?? (kind === "approved" ? "Approve because" : "Reject because")}
      </span>
      {reasons.map((reason) => (
        <button
          key={reason}
          onClick={() => (reason === "other" ? undefined : commit(reason))}
        >
          {reason.replaceAll("_", " ")}
        </button>
      ))}
      <input
        aria-label="Other reason"
        maxLength={200}
        value={note}
        onChange={(event) => setNote(event.target.value)}
        placeholder="Other reason"
      />
      <button disabled={!note.trim()} onClick={() => commit("other")}>
        Save
      </button>
      {onCancel && (
        <button className="reason-cancel" onClick={onCancel}>
          cancel
        </button>
      )}
    </div>
  );
}
