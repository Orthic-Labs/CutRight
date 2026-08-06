# Brand Voice Enforcement

MODE: OUTPUT_ONLY
PRIMARY_DELIVERABLE: Brand Card from exact granted brand source.
DISCOVERY_PROFILE: D1_SCOPED_SOURCE
EFFECT_PROFILES: source_read
SPECIALIST_REFS_MAX: 0
CHILD_AGENTS_MAX: 0
EXTERNAL_REQUESTS_MAX: 0
MAY_ADD_TASKS: NO
MAY_CALL_SKILLS: NONE
TERMINAL: Brand Card is delivered for named brand within frozen source scope.

Before writing or designing anything, you MUST load the brand card and apply it to every output.

## Usage

User invokes `cutright://skill/brand {"brand_code":"DD"}` (or RH, SS, HR, TS), or says "for [brand]" anywhere in their request.

Output a **Brand Card** at the start of your response, then proceed with the task using those rules.

---

## Damned Designs (DD)

**Category:** Premium EDC — fixed blades, fidget/desk toys, lanyard beads
**Domain:** www.damneddesigns.com
**Audience:** People who notice craftsmanship. EDC enthusiasts, designers, makers, knife collectors. Not tactical-bros, not hypebeasts.

### Voice
- **Tone:** Dark, considered, slightly dangerous. Dry wit. Quiet confidence.
- **Never:** Loud, exclamation-heavy, hype-speak, "GAME CHANGER", "10X", "INSANE", emoji fireworks, fake urgency
- **Always:** Precise nouns, working verbs, restrained adjectives. Sentences end. White space.
- **Reference voices:** Best Made Co (RIP), Filson catalog copy, Aesop product cards, A24 trailer voiceovers

### Visual System
| Element | Value |
|---|---|
| Primary accent | Copper `#B87333` |
| Background | Beige `#F7F2EA` |
| Dark base | Ink `#111110` / Near-black `#1A1A1A` |
| Light contrast | Off-white `#F5F0E8` |
| Mid-tone | Warm grey `#8A8070` |
| Display font | Cormorant Garamond (600/700/700i) |
| Body font | IBM Plex Sans (400/500/700) |
| Accent/labels | IBM Plex Mono (400) |
| Video font | Cormorant Garamond + DM Sans |

### Motion
- Slow eased-in reveals over snappy cuts
- `spring({ stiffness: 60, damping: 14 })` default
- Copper accents animate LAST as a finishing detail

### Restrictions
- **NEVER fabricate:** quotes, statistics, testimonials, personal stories, customer reviews
- Use placeholders like `[YOUR STORY]`, `[CUSTOMER NAME]`, `[VERIFIED STAT]`
- Don't claim materials, processes, or origins you can't verify

---

## Rotten Hand (RH)

**Category:** Slow fashion — ethically made clothing, anti-fast-fashion
**Domain:** rottenhand.com (no www)
**Audience:** People tired of disposable fashion. Conscious buyers, 28-45, value quality + ethics over trends.

### Voice
- **Tone:** Honest, grounded, occasionally sharp. Real. Not preachy.
- **Never:** Greenwashing buzzwords ("sustainable journey", "conscious consumer"), guilt-tripping, performative virtue
- **Always:** Specific facts (factory, fiber, wage), direct comparisons to fast fashion, concrete numbers
- **Reference voices:** Patagonia field reports, Outlier Garments, Buck Mason emails, Cuyana product pages

### Visual System
| Element | Value |
|---|---|
| Accent | Muted rose `#b07a84` |
| Accent hover | `#9d6b75` |
| Decorative | `#c49da5` |
| Display font | Fraunces (400/500) |
| Body font | Inter (400/500/600) |
| Logo font | Montserrat (300) |
| Heading tuning | `letter-spacing: -0.02em; line-height: 1.05` |
| `<em>` in headings | weight 500 (Fraunces medium) |

### Restrictions
- **NEVER fabricate** quotes, stats, testimonials, stories
- Geographic stat scope must be correct (e.g. "85% of textiles" is US-only, not global)
- Verify regulatory status against current date (today is 2026-04-27)
- Blog topics: textile science, ethical fashion, craftsmanship, repair culture

---

## Stunning Strangers (SS)

**Category:** Photography — fine art, portraiture, visual storytelling
**Domain:** stunningstrangers.com (no www)
**Audience:** Art collectors, photo lovers, gallery-goers, brands seeking unconventional portraiture.

### Voice
- **Tone:** Personal, introspective, visually driven. First-person.
- **Never:** Overwrite the image. Adjective stacks. "Capturing the essence of..."
- **Always:** Sparse, deliberate. Let the photo do the work. The yellow eyes are real (not contacts) — never explain unless asked.
- **Reference voices:** Magnum Photos captions, Aperture editorials, Sally Mann interviews

### Visual System
| Element | Value |
|---|---|
| Background | Neutral dark grey `#121212` |
| Placeholder grey | `#262626` (Instagram grey) |
| Accent / Nav | Gold `#F5C518` |
| Contact page | Cream `#fdfbf2` |
| Primary font | Space Grotesk (variable 400-700) |
| Accent font | Gelato Luxe — logo + 1-2 `<em>` accent words per section ONLY, +15-25% size bump |

### Restrictions
- **NEVER fabricate** photographer backstory, exhibition history, awards
- Yellow eyes = real feature, not lens/edit
- No fake "the artist says..." quotes

---

## Heard Right (HR)

**Category:** Software — local-first dictation app with voice commands. Superwhisper-class transcription + a hands-free command layer.
**Domain:** heardright.app
**Audience:** Knowledge workers, developers, dictation power-users who want hands-free + privacy (local-first, no cloud).

### Voice
- **Tone:** Direct without curt. Honest without preachy. Confident without smug. Warm without cute.
- **Always:** Concrete command/wake-word examples ("say *zephyr screenshot*", "*zephyr stop*"). Real numbers. Specific outcomes.
- **Never:** "revolutionary", "disruptive", "AI-powered", "game-changing", "empower", "leverage", "synergy", "unlock", "limited time", "act now", "the only", "the best" (full banned list in `heardright/brand.md`)
- **Positioning lock:** Local-first transcription + voice commands. NEVER position around accents, Indian English, Hindi-English, or "accents Western tools mangle" — that framing is retired.
- **Reference voices:** Linear changelog notes, Cron release posts, Arc product copy

### Visual System (LOCKED 2026-06-05 — Right Suite identity; source `skills/brand/brand-pack/heard-right.md` and `skills/brand/brand-pack/right-suite.md`)
| Element | Value |
|---|---|
| Accent (ember) | `#FF5630` dark / `#DF3400` light |
| Dark — bg | `#211D1A` (warm coffee, never pure `#000`) |
| Dark — border / text / muted | `#352F2B` / `#F3EEEA` / `#998F89` |
| Light — bg / surface | `#F7F3EC` (cream) / `#FFFFFF` |
| Light — border / text / muted | `#E4DCDA` / `#272322` / `#686160` |
| Display font | Tanker |
| Body font | Hanken Grotesk |
| Mono font | Spline Sans Mono |
| Logo wordmark | `HeardRight` — one word, "Right" set in ember |

### Visual rules
- ONE accent only (ember). Never a second.
- No gradients. Solid fills only.
- Cream/coffee baseline, never pure white/black.
- Each app is a dark+light theme (user-switchable in-app; websites ship light-only).
- The retired copper `#C46A3D` + Source Serif 4 / Inter / `Heard.Right` values were a Damned Designs copy-paste — never use them.

### Restrictions
- **NEVER fabricate** user stories, testimonials, productivity stats, transcription accuracy benchmarks
- Use concrete examples not abstract claims ("hold *zephyr*, say *open Slack*", not "control your apps")
- Wake word is **zephyr** (locked 2026-06-12). Retired — never propose: `decipher`, `quasar`, `cypher`, `cipher`, `shunya`.
- Positioning is local-first transcription + voice commands — NOT accent-tuned.

---

## Toxic Sundae (TS)

**Category:** Streetwear with a slow-fashion wedge. "The antidote to fast fashion."
**Domain:** toxicsundae.com
**Audience:** Counter-culture leaning. People who research before buying, anti-trend, anti-throwaway.

### Voice
- **Tone:** Defiant, direct, blunt. Counter-culture energy without preachy. Founder-led — Adrian as the voice ("A decade inside fast fashion. This is the exit.")
- **Headlines:** ALL CAPS, condensed, fragment-style. Two-beat pattern: setup + punchline ("NOT FAST FASHION. THE ANTIDOTE."). Toxic green accent on the punchline word/phrase only — never on the setup.
- **Body copy:** Short, declarative, factual. Real numbers, real geography ("150 billion garments produced every year. 65% trashed within 12 months." "Ships within 1-2 business days." "Ethically made in India · Ships from California.")
- **Founder voice (Adrian):** First person from lived experience — never abstract.
- **Never:** Vague sustainability speak ("eco-friendly", "conscious"), influencer slang, hype emoji, "limited drop", greenwashing
- **Reference voices:** Patagonia early activist copy, Vans counter-culture origin, dystopian sci-fi paperback blurbs

### Visual System
| Element | Value |
|---|---|
| Toxic green (primary accent) | `#39FF14` |
| White | `#FFFFFF` |
| Sage / muted green | `#60805D` |
| Charcoal | `#484E48` |
| Olive / khaki brown | `#6A654E` |
| Background of choice | Black (logo + pattern designed for black bg) |
| Header font | Zen Dots Regular (sci-fi / geometric / distressed) |
| Body font | Poppins (Medium / SemiBold / Regular) |
| Logo | Skull-shaped bowl with toxic-green ice cream scoops, distressed "TOXIC SUNDAE" wordmark |
| Pattern | Liquid swirl / marbled green-black-sage psychedelic flow |

### Visual rules
- Logo never altered, rotated, or recolored outside palette
- Min size: 3cm wide for full lockup, 2cm for icon
- Clear space = height of letter U around logo
- Logo can't be placed on textured backgrounds (other than the brand pattern itself)
- Forbidden: pink/purple bg, broken-up arrangements, off-palette colors

### Restrictions
- **NEVER fabricate** manufacturing claims, fair-wage stats, sustainability metrics — Adrian's actual supply chain only
- Trust signals must be specific, not vague ("Ships within 1-2 business days", not "fast shipping")
- ⚠ **Overlap with Rotten Hand**: both are Adrian's slow-fashion brands. RH = "clothes that outlast trends, scrapbook honesty." TS = "counter-culture streetwear, dystopian aesthetic." Before any cross-promotion or shared campaign, confirm audience/category differentiation.

---

## Cross-Brand Rules

1. **Never mix brand assets** — each brand has its own palette, fonts, voice. DD copper does not appear on RH; RH rose does not appear on TS; etc.
2. **Never fabricate** quotes, stats, testimonials, customer stories, or founder backstory for ANY brand.
3. **Always specify the brand** at the top of any creative output.
4. After loading the brand card, every subsequent output in this session must respect it until a different `cutright://skill/brand` invocation is made.
5. **Same founder, different voices** — Adrian founded all 5. DD = "the brand" speaking. RH = "Adrian the founder" speaking. TS = "Adrian the dissenter" speaking. HR = product/builder voice. SS = nearly silent, photographer.

## Output Format

Start every response after the `cutright://skill/brand` invocation with:

```
🎯 Brand: [DD | RH | SS | HR | TS]
Voice locked: [one-line summary]
Visual locked: [accent color, fonts]
Restrictions active: [the no-fabrication rule, scope rules]
```

Then proceed with the task.
