# Book 1 orchestrator variance log

Recorded per AGENTS.md: any variance past ±10% or outside the manifest's
task accounting is recorded here rather than billed or hidden.

## V-1: shared import tooling built by the orchestrator, not by a lane task

- What: the twelve Python helpers in `tools/import-closure/`
  (`_common.py`, `import_selected.py`, `import.py`, `hash_tree.py`,
  `verify_copy.py`, `assert_no_external_refs.py`, `rewrite_refs.py`,
  `verify_exclusions.py`, `scan_assets.py`, `validate_asset_ledger.py`,
  `verify_notices.py`, `validate_clean_room.py`) plus the `--out FILE` /
  `--source` flags on the Rust closure scanner.
- Why: lane task commands in the manifest invoke these scripts from their
  first task (A-007, B-012, C-017), but no task in any lane owns creating
  them. The Rust scanner crate is task CR-V2-B1-004 output; the Python
  helpers had no owning task.
- Ownership note: `tools/import-closure/**` is lane-c-exclusive per
  `imports/v2/ownership.json`. Lanes A and B are granted read/execute
  access to the committed helpers for the duration of Book 1.
- Verification: scanner `cargo fmt`/`clippy --locked`/`cargo test --locked`
  clean (11 tests); `--out`/`--source` smoke run wrote a deterministic
  graph; end-to-end smoke import of `tools/skills/brand` from workspace
  pin 6ee21f03a787e7b57dc412760a8996ea7a235302 into a scratch dest,
  verified by `verify_copy.py`, then removed. Scratch paths never staged.

## V-2: corpus allowed_paths do not literally exist in pinned sources

- What: several corpus `allowed_paths` entries (for example top-level
  `designer/**` under workspace-capabilities, or `engine/**`, `core/**`,
  `platform/**`, `models/**` under heardright) do not exist at those
  literal paths in the pinned revisions. The actual material lives at
  `tools/skills/*` (workspace pin 6ee21f03a787e7b57dc412760a8996ea7a235302)
  and `tauri-app-next/src-tauri/src/` (HeardRight pin
  b60bff947f12ffa9d25e94ad27e8ff30db006a24).
- Handling: lanes map corpus intent to the actual pinned paths inside
  their selection files and record the mapping in receipts; they do not
  fabricate paths. This is an honest deviation, not a waiver.

## V-3: external pins require local clones

- What: Vox (8b034354dc443edcde7fdb2622e0491df5142fd3), Palmier
  (397b82e64093f986cbabd89f1a1c93812ff546c2), and AutoShorts
  (f17b04cdd97ef65c32b81b31b36bb6eb5d013d5b) have no local checkout in
  this workspace. Clones are fetched read-only into
  `/Volumes/D/claude/tmp/v2-sources/` (outside the CutRight repository)
  and pinned by commit; nothing from them is imported except through the
  selection/receipt machinery and the corpus dispositions.
