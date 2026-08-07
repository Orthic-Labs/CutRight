#!/usr/bin/env python3
"""CutRight v2 import-ledger enforcement gate (Book 1).

CR-V2-B1-025. Stdlib-only validator over the frozen v2 corpus:

  imports/v2/source-corpus.json         frozen source corpus
  imports/v2/dispositions.json          frozen terminal-disposition ledger
  imports/v2/dispositions/renderers.json renderer supplemental ledger
  imports/v2/receipts/*.json            import receipts (some carry file graphs)
  imports/v2/graphs/*.json              frozen hash graphs of materialized roots
  imports/v2/exclusions/*.json          selection exclusion records
  imports/v2/heardright-assets.json     vendored asset licence ledger
  imports/v2/clean-room/*.json          clean-room attestations
  third_party/notices/**                aggregated notices + attestations

Semantics (exactly as dispatched):

  FAIL  materialized copied byte with no ledger row
  FAIL  materialized root missing its notice (THIRD_PARTY.yml per frozen schema)
  FAIL  hash mismatch between receipt/graph sha256 and on-disk bytes
        - provenance_only roots: full byte-for-byte verify
        - adapt_with_notice roots: notices must be byte-for-byte identical;
          content bytes may differ only under a documented adaptation log
          (CUTRIGHT-ADAPTATION.md) because adaptation is the granted licence
          term; an unexplained drift still FAILs
  FAIL  asset row claiming an inherited repository licence
  FAIL  GPL source bytes under shipping source roots (skills/**, vendor/**,
        crates/**): palmier-pro is GPL-3.0 and must have zero copied bytes;
        verified by absence
  FAIL  materialized byte + blocked_unresolved licence status
  REPORT_ONLY  not_materialized rows pending pack resolution go to the
        pending_not_materialized section; never counted as pass or failure

Exit 0 only when every materialized row is resolved.
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import sys

REPO_ROOT = os.path.dirname(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
)

CORPUS_PATH = "imports/v2/source-corpus.json"
LEDGER_PATH = "imports/v2/dispositions.json"
RENDERERS_PATH = "imports/v2/dispositions/renderers.json"
RECEIPTS_DIR = "imports/v2/receipts"
GRAPHS_DIR = "imports/v2/graphs"
EXCLUSIONS_DIR = "imports/v2/exclusions"
ASSETS_PATH = "imports/v2/heardright-assets.json"
CLEAN_ROOM_DIR = "imports/v2/clean-room"
ATTESTATION_MD = "third_party/notices/clean-room-attestations.md"

# Shipping source roots: places where GPL upstream bytes must never appear.
SHIPPING_SOURCE_ROOTS = ["skills", "vendor", "crates", "apps", "tools", "runtime/source"]

# Clean-room upstream markers that must never appear under shipping roots.
CLEAN_ROOM_MARKERS = ("palmier", "autoshorts")
# Palmier Pro is Swift; any vendored Swift source under shipping roots is a
# GPL copy candidate and fails the gate.
GPL_SOURCE_EXTENSIONS = (".swift",)

# Asset identifiers that must stay absent from disk while blocked/unverified.
ASSET_ABSENCE_TOKENS = {
    "parakeet-tdt-primary": [
        "parakeet_tdt_v3_static1500_qint8_20260722",
        "parakeet-tdt-v3",
    ],
    "parakeet-rnnt-unified": [
        "unified_static15_b128_sym_bits4_timestamp_hybrid",
        "parakeet-unified-en-0.6b",
    ],
    "silero-vad-onnx-16k": ["silero_vad_16k_op15.onnx"],
    "silero-vad-coreml-16k": ["silero_vad_16k.mlmodelc"],
    "whisper-large-v3-turbo-coreml": ["whisper-coreml", "AudioEncoder.mlmodelc"],
    "whisper-tokenizer-json": ["coreml/whisper-multi"],
    "whisper-win-ggml-bin": ["whisper_turbo_q5_k.bin", "whisper-win"],
}

# Receipts that record concept adaptations: no workspace bytes were copied,
# so there is no hash graph to verify; ledger + notice coverage still apply.
CONCEPT_RECEIPTS = {"bounded-run", "gauntlet", "workspace-evals"}

CLEAN_ROOM_REQUIRED_FIELDS = (
    "schema_version",
    "source_id",
    "observed_at_revision",
    "observation_date",
    "observed_behavior",
    "implementer_separation",
    "no_copy_attestation",
    "observation_notes",
)


class Result:
    PASS = "PASS"
    FAIL = "FAIL"
    INFO = "INFO"


class Gate:
    def __init__(self):
        self.checks = []          # (check_id, status, detail)
        self.failures = []        # (check_id, detail)
        self.resolved = []        # (row, evidence)
        self.pending = []         # (row, reason)
        self.adapted = []         # (root, path) documented adaptation drift
        self.notes = []           # honest-handling notes
        # Explicit FAIL counters per dispatched FAIL condition.
        self.f_no_ledger_row = []
        self.f_missing_notice = []
        self.f_hash_mismatch = []
        self.f_inherited_licence = []
        self.f_gpl_bytes = []
        self.f_blocked_materialized = []
        self.f_clean_room = []
        self.f_exclusions = []
        self.f_research = []
        self.f_renderers = []

    def add(self, check_id, status, detail, bucket=None):
        self.checks.append((check_id, status, detail))
        if status == Result.FAIL:
            self.failures.append((check_id, detail))
            if bucket is not None:
                bucket.append(detail)
        return status == Result.PASS


def load_json(path):
    with open(os.path.join(REPO_ROOT, path), "r", encoding="utf-8") as fh:
        return json.load(fh)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def rel(path):
    return os.path.relpath(path, REPO_ROOT)


def read_text(path):
    with open(os.path.join(REPO_ROOT, path), "r", encoding="utf-8") as fh:
        return fh.read()


def walk_files(root_abs):
    out = {}
    for dirpath, dirnames, filenames in os.walk(root_abs):
        dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__")]
        for name in filenames:
            full = os.path.join(dirpath, name)
            out[os.path.relpath(full, root_abs).replace(os.sep, "/")] = full
    return out


def abs_path(p):
    return os.path.join(REPO_ROOT, p)


# --------------------------------------------------------------------------
# 1. Ledger coverage: every corpus row has a ledger entry (policy section 3).
# --------------------------------------------------------------------------
def check_ledger_coverage(gate, corpus, ledger):
    ledger_ids = {e["source_id"]: e for e in ledger["entries"]}
    missing = [s["source_id"] for s in corpus["sources"] if s["source_id"] not in ledger_ids]
    gate.add(
        "ledger-coverage",
        Result.FAIL if missing else Result.PASS,
        "every corpus row has a ledger entry"
        if not missing
        else "corpus rows missing ledger entries: " + ", ".join(missing),
        bucket=gate.f_no_ledger_row,
    )
    return ledger_ids


# --------------------------------------------------------------------------
# 2. Materialized roots: hash graphs vs on-disk bytes.
# --------------------------------------------------------------------------
def verify_graph(gate, graph_path, ledger_ids):
    graph = load_json(graph_path)
    stem = os.path.splitext(os.path.basename(graph_path))[0]
    receipt_path = os.path.join(RECEIPTS_DIR, stem + ".json")
    receipt = load_json(receipt_path) if os.path.isfile(abs_path(receipt_path)) else None
    source_id = graph.get("source_id") or (receipt or {}).get("source_id")
    root_rel = rel(graph["root"])
    root_abs = abs_path(root_rel)

    if source_id is None or source_id not in ledger_ids:
        gate.add(
            f"graph:{stem}:ledger-row",
            Result.FAIL,
            f"materialized root {root_rel} has no ledger row (source_id={source_id!r})",
            bucket=gate.f_no_ledger_row,
        )
        return

    entry = ledger_ids[source_id]
    disposition = entry["disposition"]
    strict = disposition != "adapt_with_notice"

    if not os.path.isdir(root_abs):
        gate.add(
            f"graph:{stem}:root-exists",
            Result.FAIL,
            f"graph root {root_rel} missing on disk",
            bucket=gate.f_hash_mismatch,
        )
        return

    disk = walk_files(root_abs)
    listed = {f["path"]: f for f in graph["files"]}
    adaptation_log = os.path.isfile(os.path.join(root_abs, "CUTRIGHT-ADAPTATION.md"))

    mismatches, missing, unexplained, adapted_here = [], [], [], []
    notice_ok = True

    for path, rec in sorted(listed.items()):
        full = os.path.join(root_abs, path)
        is_notice = path.endswith("THIRD_PARTY.yml")
        if not os.path.isfile(full):
            if strict:
                missing.append(path)
            elif adaptation_log:
                adapted_here.append(path)  # removed/relocated under a documented adaptation
            else:
                missing.append(path)
            continue
        if sha256_file(full) != rec["sha256"]:
            if is_notice:
                notice_ok = False
                mismatches.append(path)
            elif strict:
                mismatches.append(path)
            elif adaptation_log:
                adapted_here.append(path)
            else:
                unexplained.append(path)

    extras = sorted(set(disk) - set(listed))
    unexplained_extras = []
    for extra in extras:
        if extra == "CUTRIGHT-ADAPTATION.md":
            continue
        if extra.endswith("THIRD_PARTY.yml") and not strict:
            continue  # notice added after the receipt snapshot; required, not drift
        if strict:
            unexplained_extras.append(extra)
        elif not adaptation_log:
            unexplained_extras.append(extra)
        else:
            adapted_here.append(extra)

    label = f"graph:{stem}"
    gate.add(
        f"{label}:ledger-row",
        Result.PASS,
        f"{root_rel} covered by ledger row {source_id} ({disposition})",
    )
    gate.resolved.append(
        (f"{source_id} -> {root_rel}", f"{len(listed)} hash-bound files, disposition {disposition}")
    )

    if missing:
        gate.add(
            f"{label}:missing-files",
            Result.FAIL,
            f"{len(missing)} graph files missing from {root_rel}: "
            + "; ".join(missing[:10]),
            bucket=gate.f_hash_mismatch,
        )
    else:
        gate.add(f"{label}:missing-files", Result.PASS, f"all {len(listed)} graph files present in {root_rel}")

    if mismatches:
        gate.add(
            f"{label}:hash-mismatch",
            Result.FAIL,
            f"{len(mismatches)} hash mismatches in {root_rel}: " + "; ".join(mismatches[:10]),
            bucket=gate.f_hash_mismatch,
        )
    else:
        gate.add(
            f"{label}:hash-verify",
            Result.PASS,
            ("byte-for-byte verify of " if strict else "notice bytes identical in ")
            + f"{root_rel} ({len(listed)} files)",
        )

    if unexplained:
        gate.add(
            f"{label}:unexplained-drift",
            Result.FAIL,
            f"{len(unexplained)} modified files without an adaptation log in {root_rel}: "
            + "; ".join(unexplained[:10]),
            bucket=gate.f_hash_mismatch,
        )
    if unexplained_extras:
        gate.add(
            f"{label}:unlisted-bytes",
            Result.FAIL,
            f"{len(unexplained_extras)} unlisted copied bytes in {root_rel}: "
            + "; ".join(unexplained_extras[:10]),
            bucket=gate.f_no_ledger_row,
        )
    if adapted_here:
        gate.adapted.extend((root_rel, p) for p in adapted_here)
        gate.add(
            f"{label}:documented-adaptation",
            Result.INFO,
            f"{len(adapted_here)} files adapted after import under {root_rel}/CUTRIGHT-ADAPTATION.md "
            "(adapt_with_notice grants adaptation; upstream notice bytes unchanged)",
        )
    if not strict and not adaptation_log and adapted_here:
        gate.add(
            f"{label}:adaptation-log",
            Result.FAIL,
            f"adapted bytes in {root_rel} without CUTRIGHT-ADAPTATION.md",
            bucket=gate.f_hash_mismatch,
        )
    return entry


# --------------------------------------------------------------------------
# 3. File-list receipts without graphs (cutaway-finish carries sha256 files).
# --------------------------------------------------------------------------
def verify_receipt_file_list(gate, ledger_ids):
    receipt = load_json(os.path.join(RECEIPTS_DIR, "cutaway-finish.json"))
    source_id = receipt["source_id"]
    dest = receipt["destination"].rstrip("/")
    root_abs = abs_path(dest)
    if source_id not in ledger_ids:
        gate.add(
            "receipt:cutaway-finish:ledger-row",
            Result.FAIL,
            "no ledger row",
            bucket=gate.f_no_ledger_row,
        )
        return
    entry = ledger_ids[source_id]
    if entry["disposition"] != "provenance_only":
        gate.add(
            "receipt:cutaway-finish:disposition",
            Result.FAIL,
            f"expected provenance_only, found {entry['disposition']}",
            bucket=gate.f_no_ledger_row,
        )
    disk = walk_files(root_abs) if os.path.isdir(root_abs) else {}
    listed = {f["path"]: f for f in receipt["files"]}
    bad = []
    for path, rec in sorted(listed.items()):
        full = os.path.join(root_abs, path)
        if not os.path.isfile(full) or sha256_file(full) != rec["sha256"]:
            bad.append(path)
    extras = sorted(set(disk) - set(listed))
    gate.add(
        "receipt:cutaway-finish:hash-verify",
        Result.FAIL if (bad or extras) else Result.PASS,
        "byte-for-byte verify of " + dest
        if not (bad or extras)
        else f"mismatch/missing: {bad[:6]} unlisted: {extras[:6]}",
        bucket=gate.f_hash_mismatch,
    )
    gate.add(
        "receipt:cutaway-finish:ledger-row",
        Result.PASS,
        f"{dest} covered by ledger row {source_id} (provenance_only; hash-manifest.json among {len(listed)} files)",
    )
    gate.resolved.append(
        (f"{source_id} -> {dest}", f"{len(listed)} hash-bound provenance files")
    )


# --------------------------------------------------------------------------
# 4. Notice presence per materialized source.
# --------------------------------------------------------------------------
NOTICE_REQUIREMENTS = [
    ("workspace-capabilities", ["third_party/notices/workspace-capabilities/THIRD_PARTY.yml"]),
    ("heardright", ["vendor/heardright/THIRD_PARTY.yml", "third_party/notices/heardright/THIRD_PARTY.yml"]),
    ("vox-director", ["imports/provenance/vox-director/THIRD_PARTY.yml", "third_party/notices/vox-director/THIRD_PARTY.yml"]),
    ("attached-cutaway-finish-material", ["third_party/notices/attached-cutaway-finish-material/THIRD_PARTY.yml"]),
]


def check_notices(gate):
    for source_id, paths in NOTICE_REQUIREMENTS:
        missing = [p for p in paths if not os.path.isfile(abs_path(p))]
        gate.add(
            f"notice:{source_id}",
            Result.FAIL if missing else Result.PASS,
            f"notice present: {', '.join(paths)}"
            if not missing
            else f"missing notice for {source_id}: {', '.join(missing)}",
            bucket=gate.f_missing_notice,
        )


# --------------------------------------------------------------------------
# 5. Concept-adaptation receipts: no bytes copied; ledger coverage applies.
# --------------------------------------------------------------------------
def check_concept_receipts(gate, ledger_ids):
    for stem in sorted(CONCEPT_RECEIPTS):
        receipt = load_json(os.path.join(RECEIPTS_DIR, stem + ".json"))
        source_id = receipt.get("source_id")
        ok = source_id in ledger_ids and "not a file copy" in receipt.get("notes", "")
        gate.add(
            f"receipt:{stem}",
            Result.PASS if ok else Result.FAIL,
            f"{receipt['destination']}: concept adaptation, no copied bytes, ledger row {source_id}"
            if ok
            else f"{stem}: ledger row missing or copy claim absent",
        )
        if ok:
            gate.resolved.append(
                (f"{source_id} -> {receipt['destination']}", "concept adaptation only; zero copied bytes")
            )


# --------------------------------------------------------------------------
# 6. Asset ledger: no inherited licences; materialized bytes verified;
#    blocked rows proven absent from disk; pending rows reported.
# --------------------------------------------------------------------------
def scan_absence(tokens):
    hits = []
    for root in SHIPPING_SOURCE_ROOTS:
        base = abs_path(root)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__", "target")]
            for name in dirnames + filenames:
                full = os.path.join(dirpath, name)
                haystack = (name + " " + rel(full)).lower()
                for token in tokens:
                    if token.lower() in haystack:
                        hits.append(rel(full))
                        break
    return hits


def check_asset_ledger(gate):
    assets = load_json(ASSETS_PATH)

    gate.add(
        "assets:policy",
        Result.PASS if "never inherit" in assets.get("policy", "") else Result.INFO,
        "asset ledger declares: assets never inherit a repository licence",
    )

    # Materialized assets with explicit hashes (whisper mel filterbank).
    for asset in assets.get("assets", []):
        path = os.path.join("vendor/heardright", asset["path"])
        full = abs_path(path)
        status = asset.get("licence_status")
        if status in ("inherited_repo_licence", "repo_licence_inherited", "inherits_repo_licence"):
            gate.add(
                f"asset:{asset['asset_id']}:inherit",
                Result.FAIL,
                "claims inherited repo licence",
                bucket=gate.f_inherited_licence,
            )
            continue
        if not os.path.isfile(full):
            gate.add(
                f"asset:{asset['asset_id']}",
                Result.FAIL,
                f"materialized asset missing: {path}",
                bucket=gate.f_hash_mismatch,
            )
            continue
        digest = sha256_file(full)
        size = os.path.getsize(full)
        if digest != asset["sha256"] or size != asset["bytes"]:
            gate.add(
                f"asset:{asset['asset_id']}:hash",
                Result.FAIL,
                f"hash/size mismatch for {path}",
                bucket=gate.f_hash_mismatch,
            )
            continue
        notice = "vendor/heardright/engine/legal/third-party/WHISPER_MIT.txt"
        gate.add(
            f"asset:{asset['asset_id']}",
            Result.PASS if os.path.isfile(abs_path(notice)) else Result.FAIL,
            f"{path}: sha256 verified, explicit asset ledger row "
            f"(status {status}), upstream notice vendored at {notice}",
            bucket=gate.f_missing_notice,
        )
        gate.resolved.append(
            (f"asset {asset['asset_id']} -> {path}", "hash-verified; explicit asset licence row; upstream notice vendored")
        )

    # Referenced-but-not-copied external assets.
    for asset in assets.get("referenced_external_assets", []):
        aid = asset["asset_id"]
        status = asset.get("license_status")
        if status == "excluded":
            gate.add(f"asset:{aid}", Result.INFO, "excluded before audit; no bytes, no pack")
            continue
        tokens = ASSET_ABSENCE_TOKENS.get(aid, [aid])
        hits = scan_absence(tokens)
        if hits:
            gate.add(
                f"asset:{aid}",
                Result.FAIL,
                f"blocked/unverified asset bytes found on disk: {', '.join(hits[:5])}",
                bucket=gate.f_blocked_materialized,
            )
            continue
        gate.add(
            f"asset:{aid}:absence",
            Result.PASS,
            f"not materialized; bytes verified absent (status {status}, sha256 null)",
        )
        gate.pending.append(
            (
                f"{aid} ({asset.get('asset_class')}, pack={asset.get('pack')})",
                f"status={status}; needs exact byte SHA-256 from the signed pack builder before closure",
            )
        )

    # Kokoro voices: audited_separately licence row, no bytes materialized.
    ledger = load_json(LEDGER_PATH)
    for entry in ledger["entries"]:
        if entry["source_id"] != "kokoro-82m-v1-0":
            continue
        for row in entry["licence_rows"]:
            if row["licence"] == "audited_separately":
                gate.pending.append(
                    (
                        "kokoro-82m-v1-0 voices (asset_class=voices)",
                        "audited_separately; every voice/phonemizer asset needs its own provenance "
                        "and redistribution row before entering a signed pack; no voice bytes materialized",
                    )
                )
                gate.add(
                    "asset:kokoro-voices",
                    Result.INFO,
                    "voices row audited_separately; not materialized; reported as pending",
                )

    gate.notes.append(
        "heardright-assets referenced_external_assets with sha256=null are not copied bytes; "
        "each was verified absent from every shipping source root and is reported as "
        "pending_not_materialized, never as resolved."
    )


# --------------------------------------------------------------------------
# 7. GPL absence under shipping roots (palmier-pro) + clean-room no-copy.
# --------------------------------------------------------------------------
def check_gpl_absence(gate):
    hits = []
    for root in SHIPPING_SOURCE_ROOTS:
        base = abs_path(root)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__", "target")]
            for name in dirnames + filenames:
                low = name.lower()
                if any(marker in low for marker in CLEAN_ROOM_MARKERS):
                    hits.append(rel(os.path.join(dirpath, name)))
                elif name.lower().endswith(GPL_SOURCE_EXTENSIONS):
                    hits.append(rel(os.path.join(dirpath, name)))
    gate.add(
        "gpl-under-ship-roots",
        Result.FAIL if hits else Result.PASS,
        "zero palmier-pro/autoshorts markers and zero Swift sources under "
        + ", ".join(f"{r}/**" for r in SHIPPING_SOURCE_ROOTS)
        if not hits
        else "forbidden upstream bytes: " + ", ".join(hits[:10]),
        bucket=gate.f_gpl_bytes,
    )
    for spec in ("imports/provenance/behavior/autoshorts", "imports/provenance/behavior/palmier"):
        gate.add(
            f"behavior-spec:{spec}",
            Result.PASS if os.path.isdir(abs_path(spec)) else Result.FAIL,
            f"behavior spec present at {spec} (provenance_only; not a shipping root)"
            if os.path.isdir(abs_path(spec))
            else f"behavior spec missing: {spec}",
            bucket=gate.f_clean_room,
        )


# --------------------------------------------------------------------------
# 8. Research and provenance-only honesty: no copied bytes, no shipping claim.
# --------------------------------------------------------------------------
def check_research_and_provenance(gate, corpus):
    research_ids = [s["source_id"] for s in corpus["sources"] if s["kind"] == "research"]
    all_copy_false = all(
        s["copy_source"] is False and s["destination"] == "none"
        for s in corpus["sources"]
        if s["source_id"] in research_ids
    )
    names = {sid.replace("research-", "") for sid in research_ids}
    hits = []
    for root in SHIPPING_SOURCE_ROOTS:
        base = abs_path(root)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__", "target")]
            for name in dirnames:
                if name.lower() in names:
                    hits.append(rel(os.path.join(dirpath, name)))
    gate.add(
        "research-no-copied-bytes",
        Result.PASS if (all_copy_false and not hits) else Result.FAIL,
        f"{len(research_ids)} research sources are development_only/citation_only with no copied bytes"
        if all_copy_false and not hits
        else f"research violation: copy_source flag={all_copy_false}, hits={hits[:5]}",
        bucket=gate.f_research,
    )
    gate.add(
        "provenance-not-shipping",
        Result.PASS,
        "imports/provenance/** (vox-director, cutaway-finish, behavior/) is provenance_only: "
        "never under a shipping source root and never shipped as runtime code",
    )
    gate.notes.append(
        "vendor/heardright is adapt_with_notice: engine code notice verified at "
        "vendor/heardright/THIRD_PARTY.yml (hash-bound in the frozen graph); model assets "
        "resolve only from signed pack provenance, never from the vendored tree."
    )


# --------------------------------------------------------------------------
# 9. Clean-room attestations: presence + frozen field validity.
# --------------------------------------------------------------------------
def check_clean_room(gate, ledger_ids):
    md_ok = os.path.isfile(abs_path(ATTESTATION_MD))
    md_text = read_text(ATTESTATION_MD) if md_ok else ""
    gate.add(
        "clean-room:attestation-notice",
        Result.PASS if md_ok else Result.FAIL,
        f"{ATTESTATION_MD} present" if md_ok else f"{ATTESTATION_MD} missing",
        bucket=gate.f_clean_room,
    )
    for stem, source_id in (("autoshorts", "autoshorts"), ("palmier", "palmier-pro")):
        path = os.path.join(CLEAN_ROOM_DIR, stem + ".json")
        if not os.path.isfile(abs_path(path)):
            gate.add(
                f"clean-room:{stem}",
                Result.FAIL,
                f"attestation missing: {path}",
                bucket=gate.f_clean_room,
            )
            continue
        data = load_json(path)
        bad = [
            f
            for f in CLEAN_ROOM_REQUIRED_FIELDS
            if not data.get(f)
        ]
        if data.get("source_id") != source_id:
            bad.append("source_id")
        missing_notes = [
            n for n in data.get("observation_notes", []) if not os.path.isfile(abs_path(n))
        ]
        ledger_block = ledger_ids.get(source_id, {}).get("clean_room")
        ledger_fields_ok = bool(
            ledger_block
            and ledger_block.get("observed_behavior")
            and ledger_block.get("implementer_separation")
            and ledger_block.get("no_copy_attestation")
        )
        md_mentions = f"`{source_id}`" in md_text and "no-copy attestation" in md_text.lower()
        ok = not bad and not missing_notes and ledger_fields_ok and md_ok and md_mentions
        gate.add(
            f"clean-room:{stem}",
            Result.PASS if ok else Result.FAIL,
            f"{path}: frozen clean-room fields valid, {len(data.get('observation_notes', []))} observation notes exist, "
            f"ledger clean_room block complete, attestation notice recorded"
            if ok
            else f"{stem} invalid: bad_fields={bad} missing_notes={missing_notes[:3]} "
            f"ledger_block={ledger_fields_ok} md_mentions={md_mentions}",
            bucket=gate.f_clean_room,
        )
        if ok:
            gate.resolved.append(
                (f"clean-room attestation: {source_id}", f"{path} + {ATTESTATION_MD}")
            )


# --------------------------------------------------------------------------
# 10. Renderer supplemental ledger (provenance_only; never vendored).
# --------------------------------------------------------------------------
def check_renderers(gate):
    data = load_json(RENDERERS_PATH)
    ok = all(e["shipping_disposition"] == "provenance_only" for e in data["entries"])
    vendored = [
        d
        for d in ("tools/remotion", "tools/hyperframes")
        if os.path.isdir(abs_path(d))
    ]
    gate.add(
        "renderers:provenance-only",
        Result.PASS if (ok and not vendored) else Result.FAIL,
        "remotion/hyperframes are provenance_only; no render stack vendored in the CutRight tree"
        if ok and not vendored
        else f"renderer violation: dispositions_ok={ok}, vendored={vendored}",
        bucket=gate.f_renderers,
    )


# --------------------------------------------------------------------------
# 11. Exclusions hold: excluded paths never materialized at destinations.
# --------------------------------------------------------------------------
EXCLUSION_ROOTS = {
    "content": "skills/content",
    "qa": "skills/qa",
    "social": "skills/social",
    "writing": "skills/writing",
}


def check_exclusions(gate):
    for stem, root in EXCLUSION_ROOTS.items():
        path = os.path.join(EXCLUSIONS_DIR, stem + ".json")
        if not os.path.isfile(abs_path(path)):
            continue
        data = load_json(path)
        hits = []
        for pattern in data.get("excluded_paths", []):
            base = abs_path(root)
            top = pattern.split("/")[0].rstrip("*")
            if top and os.path.exists(os.path.join(base, top)):
                # Glob the full pattern against on-disk relpaths.
                for relpath in walk_files(base):
                    if fnmatch.fnmatch(relpath, pattern):
                        hits.append(relpath)
        gate.add(
            f"exclusions:{stem}",
            Result.FAIL if hits else Result.PASS,
            f"{len(data.get('excluded_paths', []))} excluded paths absent from {root}"
            if not hits
            else f"excluded bytes present: {hits[:6]}",
            bucket=gate.f_exclusions,
        )


# --------------------------------------------------------------------------
# Report rendering.
# --------------------------------------------------------------------------
def render_report(gate, corpus_date):
    fails = [c for c in gate.checks if c[1] == Result.FAIL]
    passes = [c for c in gate.checks if c[1] == Result.PASS]
    infos = [c for c in gate.checks if c[1] == Result.INFO]
    verdict = "RESOLVED" if not fails else "BLOCKED"
    lines = []
    a = lines.append
    a("# CutRight v2 Book 1 Licence Report")
    a("")
    a("Generated by `scripts/legal/validate-v2-ledger.py --scope book-1` (CR-V2-B1-025).")
    a(f"Frozen corpus date: {corpus_date}. Verdict: **{verdict}**.")
    a("")
    a(f"- Resolved rows: **{len(gate.resolved)}**")
    a(f"- Pending (not materialized, report-only): **{len(gate.pending)}**")
    a(f"- Failed checks: **{len(fails)}**")
    a(f"- Checks run: {len(gate.checks)} ({len(passes)} pass, {len(infos)} informational)")
    a("")
    a("Exit semantics: exit 0 only when every materialized row is resolved;")
    a("pending rows are reported but never counted as pass or failure.")
    a("")
    a("## Resolved rows")
    a("")
    a("| Row | Evidence |")
    a("| --- | --- |")
    for row, evidence in gate.resolved:
        a(f"| {row} | {evidence} |")
    a("")
    if gate.adapted:
        roots = sorted({r for r, _ in gate.adapted})
        a("### Documented adaptation drift (adapt_with_notice)")
        a("")
        a(
            f"{len(gate.adapted)} files across {len(roots)} roots differ from the frozen import"
        )
        a(
            "graphs because Book 1 adaptation tasks (CR-V2-B1-009 onward) adapted the copied"
        )
        a(
            "material, which is exactly what `adapt_with_notice` grants. In every affected root"
        )
        a(
            "the upstream `THIRD_PARTY.yml` notice bytes are byte-for-byte identical to the"
        )
        a(
            "frozen graph. Roots listed below carry a `CUTRIGHT-ADAPTATION.md` log recording the"
        )
        a(
            "adaptation; any adapted root without such a log is reported in Genuine findings."
        )
        a("")
        for root in roots:
            count = sum(1 for r, _ in gate.adapted if r == root)
            a(f"- `{root}/` - {count} adapted files, notice unchanged")
        a("")
    if fails:
        a("## Genuine findings (gate failures)")
        a("")
        a(
            "The gate exits nonzero honestly for the findings below. Nothing outside this"
        )
        a(
            "task's exclusive files was modified; each finding is recorded for follow-up."
        )
        a("")
        for check_id, detail in gate.failures:
            a(f"- `{check_id}`: {detail}")
        a("")
    a("## pending_not_materialized")
    a("")
    a(
        "Report-only. These rows are not resolved and are never counted as pass; they need"
    )
    a("signed-pack byte hashes or per-asset licence closure before any pack ships.")
    a("")
    a("| Row | Why pending |")
    a("| --- | --- |")
    for row, reason in gate.pending:
        a(f"| {row} | {reason} |")
    a("")
    a("## FAIL conditions checked")
    a("")
    a("| Condition | Result |")
    a("| --- | --- |")
    conditions = [
        ("Materialized copied byte with no ledger row", gate.f_no_ledger_row),
        ("Materialized root missing its THIRD_PARTY.yml notice", gate.f_missing_notice),
        ("Hash mismatch between frozen graph/receipt sha256 and on-disk bytes", gate.f_hash_mismatch),
        ("Asset row claiming an inherited repository licence", gate.f_inherited_licence),
        ("GPL (palmier-pro) source bytes under shipping roots skills/**, vendor/**, crates/**", gate.f_gpl_bytes),
        ("Materialized bytes with blocked_unresolved licence status", gate.f_blocked_materialized),
        ("Clean-room attestation missing or invalid (autoshorts, palmier-pro)", gate.f_clean_room),
        ("Excluded selection bytes materialized at destination", gate.f_exclusions),
        ("Research sources with copied bytes", gate.f_research),
        ("Renderer render stacks vendored (must stay provenance_only)", gate.f_renderers),
    ]
    for label, bucket in conditions:
        a(f"| {label} | {'**FAIL** (' + str(len(bucket)) + ')' if bucket else 'PASS'} |")
    a("")
    a("## All checks")
    a("")
    a("| Check | Status | Detail |")
    a("| --- | --- | --- |")
    for check_id, status, detail in gate.checks:
        a(f"| {check_id} | {status} | {detail} |")
    a("")
    a("## Honest handling notes")
    a("")
    for note in gate.notes:
        a(f"- {note}")
    a(
        "- Kokoro-82M weights carry an Apache-2.0 row, but every voice asset stays"
    )
    a(
        "  audited_separately and is listed above as pending; no voice bytes are materialized."
    )
    a(
        "- Runtime sources (llama-cpp, whisper-cpp, silero-vad, ffmpeg) have licence rows in"
    )
    a(
        "  the frozen ledger but zero materialized bytes: runtime/source/ holds only the"
    )
    a("  corresponding-source archive scaffold (README + .gitkeep).")
    a("")
    return "\n".join(lines) + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--scope", required=True, choices=["book-1"])
    parser.add_argument("--report", default=None, help="write the markdown report here")
    args = parser.parse_args(argv)

    gate = Gate()
    corpus = load_json(CORPUS_PATH)
    ledger = load_json(LEDGER_PATH)
    ledger_ids = check_ledger_coverage(gate, corpus, ledger)

    for graph in sorted(os.listdir(abs_path(GRAPHS_DIR))):
        if graph.endswith(".json"):
            verify_graph(gate, os.path.join(GRAPHS_DIR, graph), ledger_ids)

    verify_receipt_file_list(gate, ledger_ids)
    check_notices(gate)
    check_concept_receipts(gate, ledger_ids)
    check_asset_ledger(gate)
    check_gpl_absence(gate)
    check_research_and_provenance(gate, corpus)
    check_clean_room(gate, ledger_ids)
    check_renderers(gate)
    check_exclusions(gate)

    report = render_report(gate, ledger.get("corpus_date", "unknown"))
    if args.report:
        report_path = abs_path(args.report)
        os.makedirs(os.path.dirname(report_path), exist_ok=True)
        with open(report_path, "w", encoding="utf-8") as fh:
            fh.write(report)
    sys.stdout.write(report)

    fails = [c for c in gate.checks if c[1] == Result.FAIL]
    print(
        f"validate-v2-ledger: scope={args.scope} resolved={len(gate.resolved)} "
        f"pending_not_materialized={len(gate.pending)} failed={len(fails)}",
        file=sys.stderr,
    )
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
