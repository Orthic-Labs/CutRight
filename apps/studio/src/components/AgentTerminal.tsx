import { useEffect, useRef } from "react";

export interface AgentTerminalProps { bytes: readonly number[]; attached: boolean; promptVisible: boolean; exitCode: number | null | undefined; onAttach: () => void; onDetach: () => void; onResize?: (columns: number, rows: number) => void; }
export function AgentTerminal({ bytes, attached, promptVisible, exitCode, onAttach, onDetach, onResize }: AgentTerminalProps) {
  const ref = useRef<HTMLPreElement>(null);
  useEffect(() => { ref.current?.scrollTo({ top: ref.current.scrollHeight }); }, [bytes]);
  return <section className="agent-terminal" aria-label="Native CLI terminal"><header><span>Native CLI</span><span role="status">{exitCode === undefined ? (attached ? "Attached" : "Detached") : `Exited ${exitCode ?? 0}`}</span></header><pre ref={ref} tabIndex={0} aria-live="polite">{new TextDecoder().decode(new Uint8Array(bytes))}{promptVisible ? "\n› " : ""}</pre><footer><button type="button" onClick={attached ? onDetach : onAttach}>{attached ? "Detach" : "Attach"}</button>{onResize && <button type="button" onClick={() => onResize(120, 40)}>Resize 120×40</button>}</footer></section>;
}
