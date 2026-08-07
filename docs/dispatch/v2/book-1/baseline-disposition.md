# CR-V2-B1-001 — Baseline disposition note

This note documents how the v2 baseline (`7f3e5a61c729d4d877715b9a083d13a2e5ebe277`)
was actually treated during execution. It supplements `baseline.md` (which records
the frozen source revisions and lockfile hashes) and is the answer to the audit
finding that the v2 dispatch's B1-001 abort condition appeared unmet when
inspected post-execution.

## What the dispatch says

`CutRight-v2-Dispatch-Manifest.json` task `CR-V2-B1-001` procedure step 1:

> Abort unless `git rev-parse HEAD` equals `7f3e5a61c729d4d877715b9a083d13a2e5ebe277`.

That precondition is checked **at the moment B1-001 runs**, not at any later
inspection. Subsequent commits move HEAD forward, which is the normal shape of
a successful task chain.

## Verified at execution time

`git rev-parse 0963e7d^` (parent of `CR-V2-B1-001`) returns
`7f3e5a6` — the short form of `7f3e5a61c729d4d877715b9a083d13a2e5ebe277`.
The v2 task chain was forked from exactly the pinned CutRight baseline.
B1-001 ran against the required HEAD and passed.

## HEAD movement after B1-001

HEAD advanced through 189 dispatch-task commits plus post-release fix commits
(variant scoping tightening, video-jobs DAG fix, video-agent MCP IPv6 fix).
The current HEAD at the time of this disposition note records the post-v2
state, not the original v2 baseline. That is the expected outcome of a
completed dispatch.

## Audit response

The audit finding "B1-001 abort condition fires" was a category error: it
read the post-execution HEAD against a precondition that applies only at
B1-001 execution time. The audit is correct that the post-execution HEAD
differs from the pinned baseline; the dispatch does not require them to
match after execution completes.

If a future caller needs to re-establish the v2 baseline for a new dispatch
revision, the correct reset point is the parent of `0963e7d`
(`7f3e5a61c729d4d877715b9a083d13a2e5ebe277`), not any later commit.

## Status

- Frozen baseline: `7f3e5a61c729d4d877715b9a083d13a2e5ebe277`
- v2 task chain start: `0963e7d` (parent of B1-001) at `7f3e5a6`
- B1-001 precondition: **satisfied**
- Disposition: **no amendment required to baseline.md**