# V2 — Asset request, delivery, and acceptance contracts

Frozen by **CR-V2-B5-002**. Three JSON Schemas (draft-07 subset) bind the
asset pipeline that Designers, the asset producer pool, and the
asset reviewer exchange. Every artefact in the creative pipeline is
identified by an immutable id and a hash; revisions always receive new
ids.

## 1. Asset request — `cutright.asset_request/v2`

`schemas/creative/asset-request.schema.v2.json`. Required fields:

- `kind` — one of `still_image`, `vector_graphic`, `title_card`,
  `lower_third`, `end_card`, `thumbnail`, `og_card`, `caption_track`,
  `music_track`, `sfx_track`, `voiceover_track`, `procedural_animation`,
  `transition`.
- `outputs[]` — explicit `width`, `height`, `aspect` (as `W:H`), `alpha`,
  `duration_ms`, `format`, optional `color_space`, `bit_depth`,
  `variants`. Missing size/aspect/duration/format fails.
- `protected` — `safe_zones`, `identity_locks`, `label_locks`,
  `wardrobe_locks`, `product_locks`. Missing locks fail.
- `brand` — `brand_card_ref` (id + revision) plus optional
  `style_direction_id`.
- `evidence[]` — evidence refs for any text/quote/measurement used.
- `allowed_transforms` — closed enum; transformations outside this list
  fail at validation.
- `timeline_writable: false` — the request never carries timeline or cut
  fields. Timeline mutations belong to the editorial action batch, not the
  creative request.

### Trait shape

```rust
pub struct AssetRequest {
    pub id: AssetRequestId,
    pub kind: AssetKind,
    pub purpose: String,
    pub outputs: Vec<OutputSpec>,
    pub protected: ProtectedRegions,
    pub brand: BrandCardRef,
    pub evidence: Vec<EvidenceRef>,
}
```

## 2. Asset delivery — `cutright.asset_delivery/v2`

`schemas/creative/asset-delivery.schema.v2.json`. Required fields:

- `files[]` — each file carries `role` (`source` / `preview` / `proxy` /
  `final`), `path`, `format`, `size_bytes`, `hash` (blake3) and
  `immutable: true`. The final role binds the acceptance hash.
- `provenance.generator` — one of `native_procedural`,
  `native_text_shaper`, `native_audio_graph`, `local_inference`,
  `deterministic_layout`, `user_supplied`. Cloud/remote generations are
  not accepted.
- `provenance.method` — short method string; `prompt_config` retains the
  parameters used.
- `provenance.skill_pack_id` / `skill_pack_revision` — bind to the
  signed creative pack that produced the asset.
- `provenance.model_id` / `model_revision` — bind to the local model.
- `rights.status` — `owned`, `licensed`, `user_supplied`, `cc0`,
  `cc_by`, `cc_by_sa`, `public_domain`, `needs_review`, or `rejected`.
- `rights.license_id`, optional `license_notice`, `expires_at`,
  `geo_restrictions`, `attribution`.
- `semantic_inspection` — `status` plus typed findings (codes, message,
  regions, comparison evidence ids).
- `hash_bindings` — `asset_request_hash` and `delivery_hash`.

## 3. Asset review — `cutright.asset_review/v2`

`schemas/creative/asset-review.schema.v2.json`. Acceptance rule:

```
accepted ⟺ mechanical_checks.passed
         ∧ rights_check.resolved
         ∧ every protected_region_check.passed
         ∧ semantic_intent_check.passed
         ∧ binding.asset_request_hash == asset_request_hash(payload)
         ∧ binding.delivery_hash == delivery_hash(payload)
         ∧ binding.delivery_files_hash == aggregate(files[].hash)
```

- `mechanical_checks.details[]` — per-check `{ code, passed, message }`.
- `rights_check.resolved` — false when licence is missing/expired or
  when `rights.status` ∈ {`needs_review`, `rejected`}.
- `protected_region_checks[]` — per-region `{ name, passed, regions[],
  comparison_evidence_ids[], drift_score }`.
- `semantic_intent_check` — `passed`, `purpose_match` (0..1), evidence
  refs.
- `remediation[]` — actionable next-step codes.

## 4. Immutability and revisioning

- Every accepted `asset_delivery` is immutable. New revisions produce new
  ids.
- `asset_request_revision` binds the request to a specific revision of
  the BrandCard and StyleDirection.
- A delivery's `immutable` flag is always `true`; the producer MUST NOT
  rewrite accepted bytes in place. Re-issue creates a new
  `asset_delivery_id`.

## 5. Source / preview / proxy / final

- `source` — the highest-quality authoritative file. May be a
  procedural effect plan or a captured audio segment.
- `preview` — a low-cost visual proxy used by UI.
- `proxy` — a downsampled/encoded proxy used for planning and review.
- `final` — the file that ships in the project. Final must bind the
  acceptance hash in the asset review.

## 6. Failure modes

| Symptom | Required response |
| --- | --- |
| Missing size, rights, protected zones or provenance | Reject delivery; emit `mechanical_checks` failure |
| Timeline/cut fields present | Reject at request validation (`timeline_writable: false`) |
| Hash mismatch at acceptance | Reject; emit `hash_mismatch` remediation |
| Rights unresolved | Reject; emit `rights_unresolved` remediation |
| Protected content drift | Reject; emit `protected_region_drift` remediation |

## 7. Cross-references

- Skill envelope: `docs/architecture/V2-CREATIVE-OS.md`,
  `schemas/skills/skill-request.schema.v1.json`.
- Brand/style direction schemas: `schemas/creative/brand-card.schema.v2.json`
  and `schemas/creative/style-direction.schema.v2.json` (B5-003).
- Native renderer: `docs/architecture/V2-NATIVE-RENDER-GRAPH.md`.
- Critique: `schemas/creative/creative-verdict.schema.v1.json` (B5-005).
