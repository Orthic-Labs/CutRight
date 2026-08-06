# Eval import source notice — workspace-capabilities

- source_id: `workspace-capabilities`
- repository: `/Volumes/D/claude` (workspace pin)
- revision: `6ee21f03a787e7b57dc412760a8996ea7a235302`
- licence of source at pin: MIT
- imported by task: `CR-V2-B1-019`
- import mode: **concept adaptation** — eval cases, catalogue-integrity checks,
  and topology checks were rewritten to CutRight inputs/outputs, schema names,
  and roots. No upstream file bytes, prompts, or prose were copied into this
  repository.

## Surveyed sources

Every workspace eval file surveyed at the pin is listed in
`fixtures/evals/suites/import.json` (`workspace_eval_sources`). Each entry is
either:

1. **imported** — its `source_file` appears in a case file's `source` notice
   (`fixtures/evals/cases/*.json`), with per-case `source_cases` naming the
   upstream case ids the CutRight cases were adapted from; or
2. **excluded** — an exclusion row in `fixtures/evals/exclusions.json` records
   the reason (research / SEO / email / coding-agent lanes excluded per task
   019; lanes with no CutRight counterpart; partially imported files).

`tools/v2-evals/run.py --suite import` enforces that every surveyed source is
covered by exactly one of the two paths above, that every included skill has at
least one positive and one refusal/degradation case, and that each negative
fixture demonstrates its declared failure class.
