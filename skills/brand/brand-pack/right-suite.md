# Shared Illustration Language — Orthic Labs + Right Suite (LOCKED 2026-08-02)

For explanatory hero art, section illustrations, & conceptual plates, default to `/content` illustration: hand-drawn biological-mechanical imagery with semantically legible living structures, mechanically credible actions, matte materials, & one meaningful product accent. Orthic Labs inherits each product's accent or defaults to ember `#DF6428`; every Right Suite app inherits its own locked tokens below. This shared illustration language does not replace or alter logos, icon packs, wordmarks, fonts, UI, or product screenshots.

---

## Right Suite — App Identities (LOCKED 2026-06-05)

Software apps. **Fonts + color only — logos and taglines are TBD.** (The values here supersede the copper values in the Heard Right (HR) section.) Lockup board: upstream venture asset `Content/Brand Identity/Right Suite/identity/lockups.html` (not vendored; arrives via the optional signed creative data pack).

Wordmark: one word, CamelCase, no space; **set in Tanker suite-wide (Adrian, 2026-07-16)**; the "Right" half is set in the app's accent (the suite signature). **Tanker is BOTH the wordmark and the hero/display face suite-wide (Adrian, 2026-07-20).** The earlier wording here — "per-app display fonts still govern headings; Tanker is the wordmark face only" — contradicted the ViewRight entry below ("Tanker (display + wordmark)", locked 2026-07-16) and caused real drift: HeardRight and ViewRight shipped Tanker displays while MailRight (Switzer), ScrapeRight (Khand) and CodeRight (Cabinet Grotesk) did not, so the three read as a different family. Fixed 2026-07-20 — every site now sets its display token to Tanker with the previous face as fallback. Per-app body/mono fonts are unchanged and still differ by app. Each app = a dark+light theme (user-switchable in-app; websites ship light-only, though the HR-style poster system uses dark hero/band sections). All accents are WCAG AA on their bases in both modes.

### ViewRight — document viewer / editor  *(logo + wordmark + accent RE-LOCKED 2026-07-16)*
- **Fonts**: **Tanker (display + wordmark) · Geist (body) — Adrian's lock, 2026-07-16** · Spline Sans Mono (mono). *Sentient and Switzer retired from the brand system (Sentient stays only as an in-app reader font option). Site rollout pending the workspace-scheme pick.*
- **Workspace scheme (deciding on the mockup board):** slate + graphite hybrid — canvas vs reading-surface split (A: slate `#111722` canvas / graphite `#1B1917` surfaces; B: inverse; C: flat slate control). Blue stays the ONLY accent — it is the suite's connective tissue (HR cream/coffee/ember vs VR slate/graphite/blue).
- **Accent — logo blue**: `#3FA3DE` dark / `#0C6AA5` light. Derived from the restored V-mark blue `#2880B8` (hue ~203°): brightened for the slate dark base (6.42:1 on `#111722`; slate text on accent fill 6.42:1), deepened for cream (5.24:1 on `#F7F3EC`; white on fill 5.80:1). *Supersedes `#3B82F6` / `#0066F1` — retired for VR surfaces. Source DMG artwork was aligned 2026-07-16; the currently published 0.1.12 DMG remains the older artifact until the next signed/notarized Mac rebuild.*
- **Dark**: bg `#111722` · border `#2D3037` · text `#F4EFE7` · muted `#A39A91`
- **Light**: bg `#F7F3EC` · surface `#FFFDF8` · border `#E2DAD1` · text `#211C18` · muted `#6E665F`
- **Logo**: the restored ViewRight V mark, recolored 2026-07-16 to the locked accent — blue V (`#3FA3DE`) on a dark rounded tile (the original `#2880B8` V survives only in git history; it is the hue the accent was derived from). Source of truth `viewright/src-tauri/icons/_source.png` (regenerate sizes via `make_icon.py`); site favicons/OG live at `#3FA3DE`, the app icon ships with the next build (0.1.12 still carries the old blue). **Wordmark**: `ViewRight`, Tanker, "Right" in accent. **Tagline**: TBD

### ScrapeRight — transcription / OCR  *(colors + fonts RE-LOCKED by Adrian 2026-07-05)*
- **Fonts**: Khand (display — heavy condensed) · Author (body — humanist grotesque) · JetBrains Mono (data/transcripts only). *Supersedes the Supreme/all-mono direction; JetBrains-Mono-as-body made the whole site read cold, same lesson as CodeRight. Body must be a humanist sans.*
- **Accent — marker gold** (the highlighter over documents): `#FFC24B` dark / `#845C00` light (WCAG AA on both bases; white text on the `#845C00` fill = 5.98:1). *Supersedes the neon-lime `#E8FF2E` / olive `#6F8600` — retired, never propose them.*
- **Dark (near-black)** — RE-LOCKED 2026-07-19: bg `#0F0F10` · surface `#17171A` · border `#2A2A2E` · text `#F2F0EC` · muted `#969390` · accent `#FFC24B` (11.9:1 on the base). *Supersedes espresso `#1C1712` — retired, do not reinstate. At ~8% saturation the espresso read as dirty grey rather than warm brown on a phone screen. Picked from rendered app screens against near-black, green-black, richer espresso, aubergine and violet alternatives, with gold / violet `#A78BFA` / neon `#C77DFF` accent variants; gold on neutral won because SR's accent lives in small elements (badges, tab labels, one word of the wordmark) where its ~1.7× contrast advantage over any violet actually pays. Violet accent was additionally rejected as a collision with VoiceRight's locked `#A78BFA`. SR is therefore no longer "the one warm-dark app" — that framing is retired.*
- **Light (ivory)**: bg `#FBF7EF` · surface `#FFFCF5` · elevated `#FFFFFF` · border `#E6DECF` · text `#231F1A` · muted `#6B655C` · accent `#845C00`. *Both themes ship on every platform — Adrian picked this light set from rendered app screens on 2026-07-19 against neutral-paper alternatives. The accent is necessarily deeper here: `#FFC24B` on paper is 1.5:1, and holding its hue to the 4.5:1 body floor lands on `#845C00`. Gold cannot stay bright on a light background, so if a future change wants brighter gold in light mode the answer is gold as a **fill** with dark text on it — never lightening the token past AA.*
- **Elevation ramp (iOS, 2026-07-19)**: dark base `#0F0F10` → surface `#232329` (cards, list rows) → elevated `#2E2E36` → border `#3A3A43`, muted `#A2A2AA`; light base `#FBF7EF` → surface `#FFFCF5` → elevated `#FFFFFF`. ~20 points per dark step so cards read as lifted rather than dissolving into one flat black; gold holds 9.73:1 on the dark surface. *Note: SwiftUI's system grouped background (`#1C1C1D` dark) will override these unless `.listRowBackground` is set explicitly — the ramp is only real where that is applied.*
- **Wordmark**: `ScrapeRight`, "Right" in accent, Khand. **Logo**: TBD · **Tagline**: TBD

### HeardRight — dictation + voice commands
- **Fonts**: Tanker (display) · Hanken Grotesk (body) · Spline Sans Mono (mono)
- **Accent**: `#FF5630` dark / `#DF3400` light
- **Dark**: bg `#211D1A` · border `#352F2B` · text `#F3EEEA` · muted `#998F89`
- **Light**: bg `#F7F3EC` · surface `#FFFFFF` · border `#E4DCDA` · text `#272322` · muted `#686160`
- **Logo**: TBD · **Tagline**: TBD

### CodeRight — AI code review
- **Fonts**: Tanker (display — per the suite-wide 2026-07-20 Tanker lock; supersedes Cabinet Grotesk) · General Sans (body/UI) · JetBrains Mono (code only) — *updated 2026-06-29: body must be a humanist sans, NOT mono; JetBrains-Mono-as-body made the desktop app read cold/"coding"*
- **Accent**: `#E53855` dark / `#C92642` light
- **Dark (black/grey)** — **LOCKED by Adrian 2026-08-03**: bg `#0B0B0A` · panel `#181817` · border `#2F2F2D` · text `#F2F1ED` · muted `#9A978F`. *Supersedes blue-tinted slate `#111722` / border `#2B3340` — **retired, do not reinstate**. CodeRight's dark mode is slate/graphite in the neutral sense — black and grey, no blue cast. The app never actually shipped the blue slate, but this sheet kept asserting it, and it survived in two live CodeRight code paths (onboarding pre-paint window, first-run legal gate) until 2026-08-03. Enforced in-repo by `pnpm check:locked-dark-base`.*
- **Light**: bg `#FAF9F5` · panel `#FFFFFF` · border `#E6E2D9` · text `#29261B` · muted `#656358`. *Supersedes `#F4F6F2`/`#FCFBF4`/`#D7DCD0`/`#20242C`/`#626A73`.*
- **Logo**: TBD · **Tagline**: TBD

### MailRight — Workspace / Gmail client
- **Fonts**: Switzer (display) · Sentient (body) · Spline Sans Mono (mono)
- **Accent**: `#FB2C36` dark / `#DF0022` light
- **Dark**: bg `#17181A` · border `#2A2C2E` · text `#ECEDEE` · muted `#8E9094`
- **Light**: bg `#F7F3EC` · surface `#FFF8F1` · border `#E7DED2` · text `#1A1B1C` · muted `#5E6164`
- **Logo**: TBD · **Tagline**: TBD

### VoiceRight — text-to-speech + voice cloning (LOCKED 2026-06-20)
- **Fonts**: Clash Display (display) · Satoshi (body) · Spline Sans Mono (mono)
- **Accent**: violet `#A78BFA` dark / `#7C3AED` light  *(tune to WCAG AA on both bases in-app, as the others were)*
- **Dark**: bg `#211D1A` (coffee — shared with HeardRight) · border `#352F2B` · text `#F3EEF7` · muted `#998F99`
- **Light**: bg `#F7F3EC` · surface `#FFFFFF` · border `#E7DEE4` · text `#251F26` · muted `#686068`
- **Logo**: TBD · **Tagline**: TBD · **Pairing**: the hear↔voice sibling to HeardRight (shared coffee base; ember vs violet accent)

**Dark bases (shared):** slate `#111722` (View **only** — CodeRight left the blue slate on 2026-08-03) · black/grey `#0B0B0A` (Code — LOCKED) · charcoal `#17181A` (Mail) · coffee `#211D1A` (Heard / Voice) · near-black `#0F0F10` (Scrape — re-locked 2026-07-19, replacing espresso `#1C1712`; the neutral base is what lets the marker-gold accent carry the identity alone). *Scrape and Mail are now the two neutral-dark apps and sit ~8 points apart — they are told apart by accent (gold vs red `#FB2C36`), not by base. Keep that in mind before moving either base.*
