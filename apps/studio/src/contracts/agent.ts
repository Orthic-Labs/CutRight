export type AgentProvider = "claude_code" | "codex";
export type AgentStatus = "starting" | "running" | "input_required" | "paused" | "completed" | "failed";
export interface AgentRoute { provider: AgentProvider; executable: string; model: string; guided_qualified: boolean; native_ready: boolean; }
export interface AgentEvent { id: string; kind: "assistant" | "tool" | "approval" | "progress" | "error" | "receipt"; text: string; created_at: string; }
export interface AgentSessionContract {
  binding: string; observed_revision: string; plan: string | null; turn_log_refs: readonly string[]; tool_state: Record<string, unknown>; token_budget: number; resource_budget: number;
  session_id?: string; goal?: string; provider?: AgentProvider; model?: string; status?: AgentStatus; events?: readonly AgentEvent[]; approval?: { id: string; action: string } | null; progress?: number | null; result?: string | null;
}
export const EMPTY_AGENT_SESSION: AgentSessionContract = { binding: "", observed_revision: "", plan: null, turn_log_refs: [], tool_state: {}, token_budget: 0, resource_budget: 0, status: "paused", events: [], approval: null, progress: null, result: null };
