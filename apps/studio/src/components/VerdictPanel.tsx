import { REASONS } from "../contracts/review";
import type { DecisionReason, DecisionRecord } from "../contracts/review";
import { Reason } from "./Reason";

// The verdict strip under the compare/finals viewer: segment-flag reason
// picker, the latest-verdict badge, the approve/reject reason picker, or
// the approve/reject buttons — whichever applies to the current state.
// Extracted out of App.tsx (REV2 audit decomposition) — pure move, no
// behavior change. App.tsx only mounts this while `mode` is "compare" or
// "finals" (same gate the original inline block used), so the prop is
// typed to just those two rather than the full `Mode` union.
export function VerdictPanel({
  mode,
  flagging,
  note,
  setNote,
  commitSegment,
  onCancelFlag,
  latest,
  reasonKind,
  commit,
  onCancelReason,
  onApprove,
  onReject,
}: {
  mode: "compare" | "finals";
  flagging: boolean;
  note: string;
  setNote: (value: string) => void;
  commitSegment: (reason: DecisionReason) => void;
  onCancelFlag: () => void;
  latest?: DecisionRecord;
  reasonKind: "approved" | "rejected" | null;
  commit: (reason: DecisionReason) => void;
  onCancelReason: () => void;
  onApprove: () => void;
  onReject: () => void;
}) {
  return (
    <div className="verdict">
      <div>
        {flagging && mode === "compare" ? (
          <Reason
            kind="rejected"
            title="Flag segment because"
            reasons={REASONS.segment}
            note={note}
            setNote={setNote}
            commit={commitSegment}
            onCancel={onCancelFlag}
          />
        ) : latest ? (
          <span
            className={`receipt receipt-${latest.verdict}`}
            role="status"
            aria-live="polite"
          >
            <span className="receipt-verdict">
              {latest.verdict === "approved" ? "✓" : "✕"} {latest.verdict}
            </span>
            <span className="receipt-reason">{latest.reason}</span>
            {latest.subject_blake3 && (
              <code
                className="receipt-hash"
                title={`bound to ${latest.subject_blake3}`}
              >
                {latest.subject_blake3.replace(/^blake3:/, "").slice(0, 10)}…
              </code>
            )}
          </span>
        ) : reasonKind ? (
          <Reason
            kind={reasonKind}
            reasons={REASONS[mode]}
            note={note}
            setNote={setNote}
            commit={commit}
            onCancel={onCancelReason}
          />
        ) : (
          <>
            <button className="approve" onClick={onApprove}>
              ✓ Approve
            </button>
            <button className="reject" onClick={onReject}>
              ✕ Reject
            </button>
          </>
        )}
      </div>
    </div>
  );
}
