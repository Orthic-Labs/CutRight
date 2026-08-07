# CUTRIGHT-ADAPTATION — vendor/heardright (CR-V2-B1-027)

Adaptation log for the vendored HeardRight source closure. Source: heardright
@ `b60bff947f12ffa9d25e94ad27e8ff30db006a24` (provenance: `THIRD_PARTY.yml`;
import receipt: `imports/v2/receipts/heardright-source.json`, frozen graph:
`imports/v2/graphs/heardright-source.json`).

Vendored bytes are otherwise byte-for-byte against the frozen graph; the
adaptations below are the granted `adapt_with_notice` drift and are recorded
here honestly, one row per adapted file.

## Adapted files

| File | Adaptation | Reason | Task |
|---|---|---|---|
| `engine/scripts/record-bias-fixtures.sh` | Output directory no longer defaults to a user-home path. `OUTPUT_DIR` is now the first positional argument, defaulting to `hr-bias-clips/` next to this script (`${SCRIPT_DIR}/hr-bias-clips`); the header comment documents both. Recording semantics are unchanged (same subdirectory layout `positive/` and `negative/`, same resume-by-existing-file behavior); only the default clip location moved from the user's home to the repository-local script directory so the vendored tree carries no home references. | CutRight v2 standalone boundary (§9.3): release code and vendored release material must not carry home-directory references (standalone source audit rule R04-home-ref). | CR-V2-B1-027 |
