# Content

MODE: OUTPUT_ONLY
PRIMARY_DELIVERABLE: Media-production route or bounded content artifact
DISCOVERY_PROFILE: D1_SCOPED_SOURCE
EFFECT_PROFILES: asset_read,asset_delivery_emit
SPECIALIST_REFS_MAX: 1
CHILD_AGENTS_MAX: 0
EXTERNAL_REQUESTS_MAX: 0
MAY_ADD_TASKS: NO
MAY_CALL_SKILLS: cutright://skill/brand, cutright://skill/writing
TERMINAL: Return one bounded route or artifact; do not widen scope.

Choose the lightest existing production path that can create the requested asset. Provider names are
specialist branches, not separate skills.

## Route

| Primary deliverable | Read next |
|---|---|
| One-off image, product image, raster edit, or static visual | `references/routing.md`; use `$imagegen` when available |
| Article/concept illustration, including Xiaohei/Ian style | `references/article-illustrations.md`; add `references/xiaohei-illustration-style.md` only for that style |
| Biological-mechanical illustration for Orthic Labs or Right Suite | `skills/_shared/illustrate/GUIDE.md`; use its style anchor & selected generator adapter |
| Motion graphic, animated promo/ad, composited video | `references/motion-graphics.md` |
| Remotion/React video implementation | `specialists/remotion/GUIDE.md`, then only the relevant `rules/*.md` |
| Edit captured footage into reviewed YouTube/Reels/TikTok deliverables | UPSTREAM GAP: `specialists/video-editor` is referenced by upstream but absent at the pinned revision; route unavailable until the closure material exists |
| Cinematic AI-generated shot / Seedance | `specialists/seedance/GUIDE.md` |
| Avatar, talking head, HeyGen, lipsync | `references/avatar-video.md` |
| Upscale, sharpen, denoise, or improve an existing image | `specialists/image-enhancement/GUIDE.md` |
| Reel, YouTube, TikTok, local audio/video transcription | `specialists/transcription/GUIDE.md` (upstream ScrapeRight runtime = signed runtime-pack capability, not base CutRight) |
| Hands-off product demo, walkthrough, feature tour, cursor-driven screen recording | EXCLUDED branch (see `imports/v2/exclusions/content.json`); capture routes through `cutright://skill/qa {"mode":"capture"}` via the signed runtime pack |
| Extract or download slides from an existing Instagram carousel | EXCLUDED branch (see `imports/v2/exclusions/content.json`); capture routes through `cutright://skill/qa {"mode":"capture"}` via the signed runtime pack |
| KDP/Etsy book, manuscript, cover, interior, listing, upload QA | EXCLUDED branch — not a CutRight v2 video lane (see `imports/v2/exclusions/content.json`) |

## Production contract

1. Load the relevant brand rules via `cutright://skill/brand {"brand_code":"<code>"}` before prompts or assets.
2. Follow the current guarded pipeline for paid/provider execution. Never bypass its preflight,
   approval, review, provenance, or gallery rules.
3. Run the branch's smoke checklist before a batch or expensive render.
4. Keep intermediate provider files in scratch/cache paths and place only reviewed deliverables in
   the requested output location.
5. Adrian's eyes approve visual/video output before the pipeline advances.

## Quality gates

- **Parametrize creative briefs.** Image, motion, and video briefs get parametrized on named axes
  (composition, text_weight, palette discipline, risk) per `skills/_shared/parametric-design.md`, with a
  variant spread rather than one candidate for non-trivial requests.
- **Anti-slop on embedded text.** Any caption, on-screen copy, or script text produced en route
  gets the `skills/_shared/anti-slop.md` pass in embedded mode before the artifact ships.

## Boundaries

- Website, app, dashboard, static layout system, or frontend implementation -> `cutright://skill/designer`.
- Essay, blog, email, caption, script, landing-page copy, or other words-first output -> `cutright://skill/writing`.
- Platform calendar, audience growth, posting cadence, or channel optimization -> `cutright://skill/social`.
- Carousel concept, copy, slide structure, or optimization -> `cutright://skill/social`; existing-slide capture is an excluded branch (routes through `cutright://skill/qa` capture mode).
- ScrapeGraph enrichment remains pipeline behavior inside the ScrapeRight venture runtime (signed
  runtime pack); it is not a separate skill.
