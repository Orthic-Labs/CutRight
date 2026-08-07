// apps/studio/src/contracts/agent.ts — CR-V2-B6-017.
export interface AgentSessionContract { binding: string; observed_revision: string; plan: string | null; turn_log_refs: readonly string[]; tool_state: Record<string, unknown>; token_budget: number; resource_budget: number; }
export const EMPTY_AGENT_SESSION: AgentSessionContract = { binding: "", observed_revision: "", plan: null, turn_log_refs: [], tool_state: {}, token_budget: 0, resource_budget: 0 };
