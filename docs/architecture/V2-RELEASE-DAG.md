# CutRight v2 Release DAG

This document is the canonical release DAG for CutRight v2. It freezes
the lane ownership, the serial merge order, and the public release
artefacts. It is the source of truth for the architecture-level
verification in `scripts/architecture/check_crate_dag.py`.

## 1. Phases

```text
contracts (001-006)
   ├── Lane A: feedback + autonomy (007-011)
   ├── Lane B: security + recovery (012-016)
   └── Lane C: local distribution + clean-machine QA (017-021)
        │
        ▼
merge + v1-to-v2 migration (022)
        │
        ▼
final four-lane benchmark + Studio acceptance (023)
        │
        ▼
final security / privacy / licence / supply-chain audit (024)
        │
        ▼
final SBOM + provenance + disclosure (025)
        │
        ▼
build + seal local release candidate (026)
        │
        ▼
final authoritative local gate + clean-machine proof (027)
```

## 2. Lane DAG

```text
CR-V2-B7-006 (contracts freeze)
   ├── P-A: 007 → 008 → 009 → 010 → 011
   ├── P-B: 012 → 013 → 014 → 015 → 016
   └── P-C: 017 → 018 → 019 → 020 → 021
```

Lane A, B, and C are independent. Inside each lane, tasks are sequential.
Lanes do not write to each other's exclusive paths.

## 3. Integration DAG

```text
[Lane A,B,C commits] → 022 merge+v1-to-v2 → 023 four-lane acceptance
                      → 024 release audit   → 025 SBOM/provenance
                      → 026 build+seal RC   → 027 final gate
```

## 4. Output DAG

```text
contracts → schemas/feedback, schemas/security, schemas/release,
            schemas/recovery, schemas/migrations, docs/**
lane A    → crates/video-feedback, autonomous_run, autonomy+profile panels
lane B    → crates/video-security, crates/video-recovery, trust,
            privacy, pack manager
lane C    → scripts/release, scripts/qa/v2-clean-machine, samples/v2,
            docs/user/v2, release/v2/{bundle,layout,source,sample}
serial    → release/v2/{acceptance,audit,SBOM,provenance,RC,SHA256SUMS}
            + docs/dispatch/v2/book-7/{merge-receipt,final-gate,final-manifest}
```

## 5. Public release artefacts

- `release/v2/bundle-manifest.json` — self-describing, offline-verifiable.
- `release/v2/RC-MANIFEST.json` — full candidate binding.
- `release/v2/SHA256SUMS.txt` — checksum seal.
- `release/v2/SBOM.spdx.json` — SPDX SBOM.
- `release/v2/provenance.json` — provenance graph.
- `release/v2/THIRD-PARTY-NOTICES.md` — user-facing notices.
- `docs/dispatch/v2/book-7/final-manifest.json` — final gate manifest.

## 6. Acceptance criteria

A target is supported only when every required check passes for the
exact RC. Lane isolation, parallel roots, and the no-network invariant
are explicit constraints of this DAG.
