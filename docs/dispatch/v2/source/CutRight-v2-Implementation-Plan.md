# CutRight v2 implementation plan

## 1. Authority and supersession

This plan supersedes the earlier CutRight standalone implementation package. The v2 source corpus is frozen to 2026-08-06; later source/model changes require a new corpus revision and compatibility decision.

## 2. End state

CutRight is one installable offline product with five systems: Media Kernel, Evidence and Job Plane, Embedded Creative Operating System, Studio, and Shared Capability Registry. The complete Creator offline bundle performs recorded-footage editing, repurposing, procedural explainers and anchored creative without a sibling repository, global skill, Python, Node, Ollama, system FFmpeg, browser download, cloud key or network connection.

## 3. Implementation sequence

| Book | Title | Tasks | Why it is before the next book |
| --- | --- | --- | --- |
| 1 | Reproducible Corpus, Licence Closure, and Standalone Boundary | 27 | Freeze every source and licence input, compute the relevant skill/tool closure, vendor the permitted material, and make unresolved or external runtime references impossible to ship. |
| 2 | Shared Capability Registry, Typed Actions, and Transactional Project State | 27 | Create one action and capability contract for Studio, the embedded agent, CLI, MCP and tests; make every mutation revision-bound, atomic, validated and undoable. |
| 3 | Signed Runtime Packs, Hierarchical Evidence Graph, and Durable Job Plane | 27 | Replace every system-tool and sibling-app dependency with signed CutRight packs, then build bounded multimedia evidence retrieval and content-addressed resumable jobs. |
| 4 | Benchmark-First Evaluation and Editorial Intelligence | 27 | Establish the golden corpus, deterministic and model-based evaluators, then implement editorial reasoning under measurable confidence, preservation, truthfulness and escalation constraints. |
| 5 | Embedded Creative Operating System and Native Finish Renderer | 27 | Turn the imported skills into a product-local creative system, implement Designer/brand/script/platform contracts, and replace external render technologies with a CutRight-owned native finish graph. |
| 6 | Full Studio Authoring Surface, Embedded Agent, and Optional MCP | 27 | Productize the engine as a coherent desktop workflow with corrective editing, bounded evidence inspection, one-click production, and one shared typed agent tool surface. |
| 7 | Measured Autonomy, Security Hardening, Offline Distribution, and Release Acceptance | 27 | Turn review evidence into bounded per-format autonomy, harden the local product, migrate existing projects, and prove signed offline installers on clean machines without CI or external dependencies. |

The order is mandatory. Book 4 benchmark and editorial work cannot begin against mutable runtime/source identities. Book 5 creative breadth cannot be trusted before the action/evidence/benchmark foundation. Studio authoring comes after the domain APIs. Autonomy and distribution come last.

## 4. Parallel model within a book

Tasks 001–006 are sequential and freeze contracts. Tasks 007–011, 012–016 and 017–021 form three disjoint lanes. Task 022 joins all lanes in fixed A→B→C order. Tasks 023–027 integrate, test and run the one full local gate.

A single agent may execute numeric order. Parallel execution requires already-authorised isolated checkouts and must preserve one commit per task. No task tells an agent to create branches or worktrees without current user authority.

## 5. Global completion invariants

- Every source, skill, model, asset, runtime and binary is pinned, classified, licensed and hash-bound.
- Project package and immutable revision/object graph are canonical; SQLite indexes are disposable.
- Studio, agent, CLI and MCP use one capability registry, one action vocabulary and one executor.
- Skills/models propose typed artefacts/actions; Rust validates and mutates atomically.
- Evidence is hierarchical, bounded and traceable; compact summaries do not replace source evidence.
- Planner and critic are independent; deterministic checks run first.
- Cutaway selection/timing and Finish styling remain separate, with a locked editorial revision.
- Native CutRight renderer is the shipping path. Remotion/HyperFrames are provenance/migration references only.
- Runtime resolution uses signed CutRight packs only; no PATH, user interpreter, sibling application or network fallback.
- No hosted CI. `scripts/gate.sh --with-qa` remains the authoritative repository gate and runs once at the end of each book.
- A clean-machine blocked-network proof is required for every claimed desktop target.

## 6. Dispatch size

- 7 books
- 27 tasks per book
- 189 tasks total
- 15 parallel-lane tasks per book (5 in each of three lanes); 12 tasks are sequential.
- Each task has exact dependencies, ownership, commands, acceptance, stop-loss ceilings and commit message.

## 7. Release boundary

The dispatch ends with a sealed local release candidate and checksum manifest. Upload, public release, tags, announcements, account mutation and spend are intentionally excluded.
