# CutRight v2 Embedded Agent UX, Session, Planning, and Approval Policy

## 1. Purpose

This document freezes the embedded-agent UX for v2 Studio. It defines the
session shape, the bounded conversation contract, the plan/evidence/tool
triple, and the explicit approval policy. It is owned by the serial freeze
task `CR-V2-B6-005` and Lane C (`CR-V2-B6-017` through `CR-V2-B6-021`).
Lanes A and B must not redefine it inside their exclusive paths.

## 2. Authoritative schemas

- `schemas/agent/turn.schema.v1.json` — one bounded conversation turn.
- `schemas/agent/plan.schema.v1.json` — a Director-authored plan envelope.
- `schemas/agent/tool-result.schema.v1.json` — result of one tool call.

All three are closed (`additionalProperties: false`) and versioned. The
embedded agent refuses any value whose `schema` field does not match the
running version.

## 3. Session binding

```ts
type AgentSession = {
  binding: { project_id: string; timeline_id?: string; revision: string };
  observed_revision: string;
  plan?: AgentPlan;
  turn_log_refs: string[];     // pointers into persisted turn log
  tool_state: Record<string, unknown>;
  token_budget: { used: number; limit: number };
  resource_budget: { max_turns: number; max_tool_calls: number; max_wallclock_ms: number };
};
```

The agent **cannot act** without a project/revision binding. Cross-project
tool calls (where `evidence_ref.project_id` differs from the session's
`project_id`) are rejected by the executor.

## 4. Conversation contract

Each `AgentTurn` records:

- the role (`user`, `system`, `assistant`, `tool`),
- the `content` (outcome-first, concise),
- the `evidence_refs` the turn inspected (so the conversation is
  auditable),
- any `tool_calls` and their `result_ref`s,
- an explicit `outcome` enum value.

Hidden chain-of-thought is **never** stored in `content` or in any
project artefact. The planner's scratchpad lives only in running session
memory and is dropped on compaction unless the user opted to keep a
summary (which is then written as a `summary` turn, not raw reasoning).

## 5. Planning policy

Every multi-stage production or generation task must:

1. issue a Director-authored `AgentPlan`,
2. surface it to the user for approval (the planner sets
   `requires_user_approval: true`),
3. collect bounded evidence through the listed `evidence_queries`,
4. call only tools whose `schema_hash` matches the running capability
   registry,
5. propose action summaries that map to registered verbs.

Reversible corrective edits covered by the current review policy may run
without a fresh approval gate, but they still emit a plan whose
`requires_user_approval` is `false` and is recorded in the turn log.

## 6. Outcome-first communication

Agent output is always:

- the outcome ("I trimmed the first 1.2 s of dead air"),
- the evidence ("transcript window 0–8 s of source src_001"),
- one focused question if escalation is needed ("Keep the cut at
  1.2 s or push to 1.5 s?").

The agent **never**:

- narrates internal chain-of-thought,
- fabricates completed actions,
- embeds cost / spend / network actions,
- references hidden planner scratchpads.

## 7. Approval gates

| Task class                              | Approval gate       |
| --------------------------------------- | ------------------- |
| Multi-stage production / generation     | required before any tool |
| Single corrective edit (low-risk)       | inline confirmation only |
| Read-only investigation                 | none                |
| Replace / rerun / regenerate asset      | required before tool |

A blocked or failed tool call stops the agent and surfaces the typed
reason. The session never silently retries without re-reading the relevant
evidence.

## 8. Lane ownership

Lane C owns the agent session (`crates/video-agent/src/session.rs`), the
planner (`planner.rs`), the executor (`execution.rs`), the
communication style (`communication.rs`), the optional MCP surface
(`crates/video-agent/src/mcp/**`), and the AgentPanel UI. Lanes A and B
must not introduce a second agent runtime; everything must route through
the shared executor and the capability registry.

## 9. Anti-promises

- No cost / spend / network actions in offline v2.
- No chain-of-thought or hidden reasoning in any artefact.
- No tool call without a registered `schema_hash`.
- No project / revision binding-less action.
- No silently retried tool call after a `blocked` outcome.