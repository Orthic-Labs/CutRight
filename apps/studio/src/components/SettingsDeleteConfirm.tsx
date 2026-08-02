import { useRef } from "react";
import { useFocusTrap } from "../hooks/useFocusTrap";

// The REV2 §15.6 retention/deletion action, gated behind an explicit modal
// confirmation (matches CommandPalette/Help/DecisionsLedger's dialog
// pattern) since it is irreversible on this machine: it resets consent to
// off, budget to $0, and removes any cached cloud analysis output for this
// project.
export function SettingsDeleteConfirm({
  close,
  onConfirm,
}: {
  close: () => void;
  onConfirm: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(true, ref);
  return (
    <div
      ref={ref}
      className="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Delete cloud settings and cache"
      tabIndex={-1}
    >
      <div>
        <button aria-label="Cancel" onClick={close}>
          ×
        </button>
        <h2>Delete cloud settings &amp; cache?</h2>
        <p>
          Resets consent to off, budget to $0, and removes any cached cloud
          analysis data for this project. This cannot be undone.
        </p>
        <button className="reject" onClick={onConfirm}>
          Delete
        </button>
        <button className="reason-cancel" onClick={close}>
          Cancel
        </button>
      </div>
    </div>
  );
}
