# Cross-Skill Handoff Contracts

**Spec for the implementation agent.** Written 2026-07-31 against the `a8d4584` tree + REV2.
Design authority: `/social`, `/writing`, `/designer`, `/motion` own their side of each contract; the
Video Editor skill (`skills/content-video-editor/`) emits these records. Adrian's eyes approve the
visual result; nothing here overrides that taste gate.

## 0. The contract (lead)

The Video Editor owns the cut. The words, the thumbnail, the platform packaging, and any *new* motion
language belong to other skills. Each crossing is a **typed, file-backed handoff record** written into
the project package, so it is reproducible and no specialist silently edits another's locked files.

- **Location:** one JSON record per handoff at `brief/handoffs/<to>-<kind>.json` inside the
  `.video-project/` package, plus an append-only index at `brief/handoffs/manifest.json`. (This adds a
  `brief/handoffs/` directory to the canonical package layout in the vision plan §5.)
- **Direction:** `outbound` = Video Editor → specialist (a request); `inbound` = specialist → Video
  Editor (constraints or a delivered asset). Inbound records are written by the specialist and read here.
- **Rule:** a handoff names what the recipient `may_change` and what is `locked`. A recipient never
  writes outside its `may_change` set. Cut points, source ranges, and the timeline are always locked
  against non-editor skills.

### Common envelope

Every record wraps a purpose-specific `payload` in this envelope:

```json
{
  "schema_version": 1,
  "handoff_id": "ho-2026-07-31-0001",
  "from": "video-editor",
  "to": "designer",
  "project_id": "myvideo-9f2c1a2b",
  "created_at": "2026-07-31T10:15:00Z",
  "direction": "outbound",
  "purpose": "Thumbnail keyed to the payoff moment",
  "may_change": ["brief/handoffs/designer-thumbnail.json", "assets/thumbnails/"],
  "locked": ["sources/", "edit/timeline.json", "edit/variants/", "render/finals/"],
  "expected_outputs": ["assets/thumbnails/youtube.png"],
  "status": "requested",
  "payload": { }
}
```

`status` lifecycle: `requested → acknowledged → delivered → accepted | rejected`. A `rejected` record
carries a `reason` and routes back to the requester. The index `brief/handoffs/manifest.json` is
`{"schema_version":1,"handoffs":[{"handoff_id","to","kind","status","path"}]}`.

---

## 1. → Designer (styleframes / thumbnails)

**Purpose:** request a static visual — a YouTube thumbnail, a styleframe for a section, or an OG/social
card — keyed to a real finished frame. Designer owns the static layout and visual system; it does not
re-cut or re-time anything.

**File:** `brief/handoffs/designer-<kind>.json` (`kind` ∈ `thumbnail | styleframe | og-card`).
**Direction:** outbound request; Designer delivers to `assets/thumbnails/` or `assets/styleframes/`.

```json
{
  "payload": {
    "kind": "thumbnail",
    "deliverable": "youtube-thumbnail",
    "aspect": "16:9",
    "output_size": [1280, 720],
    "source_frame": {
      "output_ms": 42100,
      "variant": "natural",
      "candidate_frame_path": "cache/frames/reframe-segment-004.jpg",
      "reason": "payoff moment, subject expressive, high saliency"
    },
    "title_text": "I cut 40 minutes to 8",
    "title_source": "writing-copy handoff (or placeholder)",
    "brand_ref": "brief/VIDEO-BRAND.md",
    "safe_zones": {"keep_clear": ["bottom-center"], "reason": "platform timecode + caption burn"},
    "subject": {"box": [0.33, 0.12, 0.29, 0.55], "must_remain_legible": true},
    "references": ["assets/styleframes/prior-approved.png"],
    "constraints": ["no fabricated faces", "no stock cliché", "brand palette only"]
  }
}
```

**Acceptance:** delivered asset matches `output_size`/`aspect`, respects `safe_zones` and `subject`,
uses brand palette, and Adrian approves it. Designer records its own `draft|ship` mode; a shipped
thumbnail runs the Designer ship spine, not this skill's gates.

---

## 2. → Writing (titles / descriptions / hooks)

**Purpose:** request the words — per-platform titles, descriptions, and hooks — derived from the output
transcript and brief. Writing owns wording; it never moves a cut point or invents proof.

**File:** `brief/handoffs/writing-copy.json`.
**Direction:** outbound request; Writing delivers to `brief/copy/<platform>.json`.

```json
{
  "payload": {
    "deliverables": [
      {"platform": "youtube", "kinds": ["title", "description", "chapters"]},
      {"platform": "reels", "kinds": ["hook_line", "caption"]},
      {"platform": "tiktok", "kinds": ["hook_line", "caption"]}
    ],
    "source": {
      "output_transcript": "edit/output-transcript-natural.json",
      "packed_transcript": "analysis/transcript-packed.md",
      "final_duration_ms": 512000
    },
    "platform_constraints_ref": "brief/platform-brief.json",
    "hard_limits": {
      "youtube_title_chars": 100,
      "meta_description_chars": 155,
      "reels_caption_chars": 2200
    },
    "brand_voice_ref": "brief/VIDEO-BRAND.md",
    "proof_constraints": ["no fabricated quotes/stats/testimonials", "claims must trace to the transcript"],
    "hook_goal": "stop a scrolling editor in <3s with a specific outcome"
  }
}
```

**Acceptance:** every deliverable fits its `hard_limits` (Writing counts and trims — LLMs overshoot),
passes Writing's anti-slop gate, and every claim traces to the transcript. Writing's contract
(`/writing`) governs voice + proof; this skill only supplies the source + limits.

---

## 3. ↔ Social (platform packaging)

**Purpose:** two directions. **Inbound:** Social supplies the platform constraints the edit must hit
(`brief/platform-brief.json`). **Outbound:** the Video Editor supplies the packaged deliverables +
per-platform targets for distribution. Social owns platform, audience, hook goal, CTA, cadence; it does
**not** choose source cut points.

**Inbound file:** `brief/platform-brief.json` (written by Social, read by rough-cut/shorts/export).

```json
{
  "schema_version": 1,
  "from": "social",
  "platforms": [
    {
      "platform": "youtube",
      "aspect": "16:9",
      "target_duration_ms": [480000, 720000],
      "hook_goal": "promise the payoff in the first 15s",
      "cta": "subscribe + comment the editor's biggest time sink",
      "audience": "solo creators editing their own talking-head videos",
      "caption": "sidecar_and_optional_burned"
    },
    {
      "platform": "reels",
      "aspect": "9:16",
      "target_duration_ms": [15000, 90000],
      "hook_goal": "cold-open on the strongest claim",
      "cta": "save for the next edit",
      "caption": "burned"
    }
  ]
}
```

**Outbound file:** `brief/handoffs/social-package.json`.

```json
{
  "payload": {
    "deliverables": [
      {
        "platform": "youtube",
        "media_path": "exports/youtube/youtube.mp4",
        "caption_path": "exports/captions/youtube.srt",
        "aspect": "16:9",
        "duration_ms": 512000,
        "caption_burn": false,
        "selected_variant": "natural"
      },
      {
        "platform": "reels",
        "media_path": "exports/vertical/reels.mp4",
        "caption_path": "exports/captions/reels.srt",
        "aspect": "9:16",
        "duration_ms": 58000,
        "caption_burn": true,
        "selected_variant": "tight",
        "source_short_id": "short-002"
      }
    ],
    "copy_ref": "brief/copy/",
    "thumbnail_ref": "assets/thumbnails/youtube.png",
    "qa_report": "qa/report.json"
  }
}
```

**Acceptance:** each deliverable matches its inbound `platforms[]` target (aspect, duration, caption
burn); copy + thumbnail are attached by reference; QA passed. Social's hard gates (platform-native,
hook, proof, anti-slop, series) govern distribution; this skill guarantees the media conforms to the
inbound brief.

---

## 4. → Motion (motion-language requests)

**Purpose:** request editorial motion for a finish slot. Stock motion (a registered punch-in, a cutaway,
a transition) is applied by the Video Editor directly per [MOTION-LANGUAGE.md](MOTION-LANGUAGE.md). A
**new signature motion** — something not in the grammar — is handed to Motion, which owns the cinematic
motion language. Motion does not re-cut; it authors motion over a locked output range.

**File:** `brief/handoffs/motion-<slot_id>.json`.
**Direction:** outbound request; Motion delivers `brief/motion-plan.md` + `brief/motion-gate.json`
(Motion's contract output) and the rendered slot media.

```json
{
  "payload": {
    "slot_id": "slot-007",
    "trigger": "payoff",
    "output_range_ms": [41800, 44600],
    "variant": "natural",
    "intent": "land the reveal: a single motivated push-in that ends exactly on the spoken payoff word",
    "candidate_effect": "new-signature",
    "stock_alternative_considered": "punch-in.emphasis.v1 (rejected: too generic for the brand beat)",
    "register": "product",
    "brand_motion_ref": "brief/motion-plan.md",
    "constraints": {
      "max_patterns_in_range": 1,
      "respect_reduced_motion": true,
      "no_subject_occlusion": true,
      "caption_safe": true
    },
    "evidence": ["analysis/evidence/filmstrips/candidate-004.png"]
  }
}
```

**Acceptance:** Motion declares a register, applies its restraint test (product) or choreography test
(showpiece), and writes `motion-gate.json` with `verdict:"pass"` backed by rendered prototype evidence
(Motion's 2026-07-17 dead-pin rule: a plan alone is not a pass). The slot renders in the declared
aspect(s) without occluding subject/captions. Until the effect renderers are wired (Phase 5), Motion
authors the plan + a reference render; the engine composes it when the slot renderer lands.

---

## 5. Sequencing + invariants

| Handoff | Earliest it can be emitted | Blocks |
|---|---|---|
| → Social (inbound brief) | before rough-cut | rough-cut/shorts target selection |
| → Writing | after a locked output transcript | title/description delivery |
| → Designer | after a finished frame exists | thumbnail/styleframe delivery |
| → Motion | after finish-plan slot authored | that slot's render |
| → Social (outbound package) | after QA pass | distribution |

Invariants:

1. Cut points, source ranges, `edit/timeline.json`, and `edit/variants/` are **locked** against every
   non-editor skill. A handoff that lists them in `may_change` is invalid.
2. Every record is schema-valid and written atomically inside the package; the manifest index is
   append-only.
3. No fabricated proof crosses a handoff: Writing's claims trace to the transcript; Designer uses real
   frames; Social's targets come from the inbound brief.
4. A `rejected` handoff routes back with a `reason`; it is never silently dropped.

## Engine gaps to know

- The engine does **not** generate these records. `videoctl package social` only copies media into
  `exports/`; the typed records above are authored by the Video Editor skill (and the inbound
  `platform-brief.json` by Social). Automation of record emission is a later phase.
- `brief/handoffs/` is an addition to the canonical package layout (vision §5); adopt it in the package
  schema + migrations when this contract is implemented.
- The Motion slot renderer is Phase 5; until then a Motion handoff yields a plan + reference render, not
  a composited slot (see [MOTION-LANGUAGE.md](MOTION-LANGUAGE.md)).
