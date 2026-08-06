---
name: designer-static
description: >
  Static creative owner — flyers, social posts, OG images, banners, ad creatives, print, packaging inserts.
  Routes websites + product/app UI → cutright://skill/designer, frontend critique → cutright://skill/qa {"mode":"visual_review"},
  brand systems → cutright://skill/brand-identity. Do NOT use for interactive app/web design or visual QA.
argument-hint: "flyer | social | OG | banner | print | ad creative | <medium>"
---

# Static Creative

`cutright://skill/designer static` owns static creative artifacts: flyers, social posts, OG images, banners, ad creatives, posters,
packaging inserts, print, lookbook spreads. It is NOT the primary route for interactive UI or web pages.

## Hard routing guard — delegate first

| Request type | Route to |
|---|---|
| Product/app/dashboard UI design | `cutright://skill/designer` |
| Marketing site, landing page, web page | `cutright://skill/designer` |
| Frontend visual review / polish / QA | `cutright://skill/qa {"mode":"visual_review"}` (owns the strict rendered frontend/UI audit gate) |
| Brand system, identity, color/type lock | `cutright://skill/brand-identity` |
| Blog/SEO page | `cutright://skill/writing {"mode":"blog"}` + `the SEO technical checklist (upstream `/seo` skill excluded from the CutRight corpus; run as a manual audit)` |
| Flyer, social post, OG image, banner, ad creative, print | **this skill → `references/marketing.md`** |

When the request is ambiguous, ask which output type before loading any reference.

## Static creative workflow

1. **`cutright://skill/brand {"brand_code":"<DD|RH|HR|TS|SS>"}`** — load palette, fonts, restrictions
2. **Identify medium + dimensions** — see `references/marketing.md` cheat sheet
3. **Identify purpose:** awareness / click / save / share / screenshot
4. Read `references/marketing.md` and follow its workflow

## Internal council (static creative only)

| Reference | Role pass |
|---|---|
| `marketing.md` | Brand lead, conversion designer, copy strategist, visual director, production/spec checker |

Output standard: spec recap, 3 variant briefs, generated assets (or prompts), filenames, posting-ready note.

## Optional external jury (explicit opt-in only)

Run this jury review only when the user explicitly requests it.

CutRight adaptation: the workspace ran an opt-in jury through the workspace
tool `tools/lib/auto-jury.mjs`, which is not vendored into CutRight. The
CutRight-local equivalent is the typed visual-review action:

```
cutright://skill/qa {"mode":"visual_review","kind":"design","artifact":"<artifact path>","brand_code":"<code>","fail_hard":true,"notes":"design output"}
```

The verdict is returned as a `VisualReviewResult` typed artefact. No external
jury script ships with this skill.
