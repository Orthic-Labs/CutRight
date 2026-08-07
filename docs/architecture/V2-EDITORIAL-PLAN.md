# CutRight v2 Editorial Plan

The EditorialPlan is the schema-bound contract between the planning
subsystem (deterministic candidates + narrative Director) and the action
kernel that compiles timelines and renders. It is also the contract the
independent critic reasons over.

## 1. Required shapes

```text
pub struct EditorialBeat {
    pub beat_id: BeatId,
    pub label: BeatLabel,
    pub selected_take: CandidateId,
    pub alternates: Vec<TakeScore>,
    pub confidence: f32,
    pub evidence: Vec<EvidenceRef>,
}

pub struct EditorialPlan {
    pub plan_id: PlanId,
    pub source_revision: RevisionId,
    pub evidence_graph_revision: EvidenceGraphRevision,
    pub policy_version: PolicyVersion,
    pub beats: Vec<EditorialBeat>,
    pub order: Vec<BeatId>,
    pub reorder_logs: Vec<ReorderLog>,
    pub escalations: Vec<Escalation>,
    pub drop_reasons: Vec<DropReason>,
    pub chronological_status: ChronologyStatus,
    pub review_flags: Vec<ReviewFlag>,
}
```

## 2. Labels

Beat labels are drawn from a fixed vocabulary and depend on the chosen
arc:

- `hook`, `cold_open`, `setup`, `context`, `escalation`, `turn`
- `payoff`, `resolution`, `cta`, `summary`
- `evidence`, `quote`, `demo`, `transition`

A label outside the vocabulary is rejected at validation.

## 3. Reorder rules

A reorder is allowed only when:

- The plan records a `ReorderLog` with `from_index`, `to_index`,
  `reason`, and `claim_dependencies`.
- `chronological_status` is `truthful` or `truthful_with_disclosed_reorder`.
- A truthfulness check (see B4-018) approves the dependency graph.

A reorder without a `ReorderLog` or `chronological_status` fails validation.

## 4. Drop reasons

Drop reasons use a fixed vocabulary:

- `low_evidence`, `contradictory`, `redundant`, `out_of_arc`,
- `slate_or_setup`, `handling_only`, `low_signal`, `manual_required`.

The vocabulary is the only source of truth — tests never invent their own
reason strings.

## 5. Critical invariants

- A plan cannot contain an unknown `candidate_id` or unbound `range`.
- The canonical transcript is preserved verbatim; plan mutations never
  delete words from it.
- The canonical transcript reference is `drop_reasons` + `selected_take`
  must point to material that still exists in the source.
- A plan with missing `escalations` is invalid when any escalations were
  raised during planning.
