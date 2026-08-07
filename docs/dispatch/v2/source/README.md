# CutRight v2 implementation package

**Frozen corpus date:** 2026-08-06  
**Status:** This package supersedes every earlier CutRight standalone plan and dispatch generated in this conversation.

## What changed from v1

The v1 product boundary was correct, but the source closure, evidence architecture, runtime specificity, licensing posture, and benchmark order were not complete enough. V2 makes five corrections:

1. A reproducible, pinned source corpus replaces the unbounded claim that every possible source was reviewed.
2. The complete relevant workspace capability closure is classified and imported mechanically. An unclassified reference fails the build.
3. A hierarchical evidence graph and durable job plane become foundational systems rather than late-stage features.
4. Models, runtimes, packs, licences, hashes, supported targets, and degradation rules are explicit release inputs.
5. Benchmarks and golden projects are built before editorial autonomy or creative breadth.

V2 also rejects Remotion and HyperFrames as shipping runtimes. Existing work remains migration evidence, but the product renderer is a CutRight-owned declarative native render graph.

## Files

- `CutRight-v2-Source-Corpus-and-Ledger.md` — exact corpus, dispositions, transitive skill closure, and licence rules.
- `CutRight-v2-Product-Architecture.md` — final product, five-system architecture, data model, agent loop, UX, and non-goals.
- `CutRight-v2-Runtime-Model-Matrix.md` — runtime packs, exact selected sources, qualification rules, and clean-machine contract.
- `CutRight-v2-Benchmark-Evaluation-Plan.md` — golden corpus, metrics, gates, independent critics, and autonomy thresholds.
- `CutRight-v2-Implementation-Plan.md` — seven-book implementation order and integration policy.
- `CutRight-v2-Dispatch-Book-01.md` through `07.md` — 189 mechanical tasks, 27 per book.
- `CutRight-v2-Complete-Implementation-Dispatch.md` — all documents and all books in one file.
- `source-corpus.json`, `capability-registry.example.json`, and `runtime-packs.example.toml` — machine-readable contracts.
- `CutRight-v2-Dispatch-Manifest.json` and `CutRight-v2-Dispatch-Checklist.csv` — execution indexes.

## Non-negotiable product boundary

The installed application must work with the network blocked and an empty user `PATH`. It may use operating-system APIs, drivers, and the application files it ships. It may not require CodeRight, the workspace repository, HeardRight as a separately installed application, global skills, Python, Node, Ollama, system FFmpeg, a browser download, or a cloud service.

A complete offline installer may be split into signed first-party packs for download-size reasons. That is still one CutRight product: every required pack is built, signed, versioned, verified, repairable from the offline installer payload, and has no sibling-repository dependency.
