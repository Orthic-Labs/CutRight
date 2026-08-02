import { useRef } from "react";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { MODE_LABEL, MODE_ORDER, type Mode, type Source } from "../types";

export function CommandPalette({
  modes,
  sources,
  close,
  selectMode,
  selectSource,
  approve,
  reject,
}: {
  modes: Record<Mode, boolean>;
  sources: Source[];
  close: () => void;
  selectMode: (mode: Mode) => void;
  selectSource: (index: number) => void;
  approve: () => void;
  reject: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(true, ref);
  return (
    <div
      ref={ref}
      className="dialog command-palette"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabIndex={-1}
    >
      <div>
        <button aria-label="Close commands" onClick={close}>×</button>
        <h2>Commands</h2>
        <div className="command-list">
          {MODE_ORDER.map((mode) => (
            <button key={mode} disabled={!modes[mode]} onClick={() => { selectMode(mode); close(); }}>Switch to {MODE_LABEL[mode]}</button>
          ))}
          <button onClick={() => { approve(); close(); }}>Approve current review</button>
          <button onClick={() => { reject(); close(); }}>Reject current review</button>
          {sources.map((source, index) => (
            <button key={source.source_id} onClick={() => { selectSource(index); close(); }}>Source {index + 1}: {source.display_name ?? source.source_id}</button>
          ))}
        </div>
      </div>
    </div>
  );
}
