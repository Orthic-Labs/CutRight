import type { Mode, Source } from "../types";

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
  return (
    <div className="dialog command-palette" role="dialog" aria-modal="true" aria-label="Command palette">
      <div>
        <button aria-label="Close commands" onClick={close}>×</button>
        <h2>Commands</h2>
        <div className="command-list">
          {(["sources", "compare", "finals", "qa"] as Mode[]).map((mode) => (
            <button key={mode} disabled={!modes[mode]} onClick={() => { selectMode(mode); close(); }}>Switch to {mode === "qa" ? "QA" : mode}</button>
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
