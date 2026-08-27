# cutright-effects — Remotion render backend

REV2 plan §15.3 Phase 5: the real Node/Remotion renderer behind
`EffectRenderer::Remotion` in `crates/video-project/src/effects.rs`. This
package renders four of the five typed-registry starter effects as real
composited React/Remotion output (real typography, not `drawbox`
placeholders): `lower-third.identity-card.v1`, `stat-counter.v1`,
`quote-card.v1`, `cta-end-card.v1`. The fifth, `caption.bold-karaoke.v1`,
renders through the `ass` renderer (libass, in `crates/video-media`) instead
— per `skills/content-video-editor/workflows/finish.md`, ASS is the fast
deterministic path for fixed karaoke/phrase captions, Remotion is for
branded kinetic motion.

## Renderer contract (Rust → Node boundary)

The Rust side never spawns Node directly with a bare `Command`. Every
invocation goes through `video_core::process_runner` (`ProcessSpec` +
`run_process`, reused via `video-media`'s `run_media_command`): an explicit
`env_allow` list (`PATH`, `HOME`, `TMPDIR` — no full environment
inheritance), a mandatory duration-scaled timeout, and byte-capped
stdout/stderr. `scripts/render.mjs` is the one CLI entry point Rust calls:

```
node scripts/render.mjs bundle
node scripts/render.mjs probe   [--composition <effect-id>]
node scripts/render.mjs preview --composition <effect-id> --props-file <path> \
  --output-dir <dir> --motion <true|false> [--duration <secs>] [--width <n>] [--height <n>]
```

`preview` always writes `still.png`; when `--motion true` it additionally
writes `motion.mp4` (full animation) and `motion-reduced.mp4` (the
`prefers-reduced-motion` static-fallback variant, driven by a `reducedMotion`
prop the CLI injects — never present in a registry entry's own
`props_schema`, so it never affects Rust-side prop validation).

Props are validated against the registry entry's `props_schema` in Rust
*before* Node is ever launched (`EffectRegistry::validate_props` in
`crates/video-project/src/effects.rs`, called ahead of the renderer match).
Each composition also declares a matching Zod schema (`src/schemas.ts`) as a
second, independent validation layer inside the Remotion Studio/render path.

## Toolchain requirements

- Node version pinned by the repo root `.node-version` (`26.8.1`);
  `packageManager` here is pinned to the same `pnpm@11.24.0` as
  `apps/studio`.
- `pnpm install` in this directory before any render — the Rust renderer
  checks for `node_modules/` and fails with a named remediation
  (`pnpm --dir apps/effects install`) if it is missing, rather than silently
  falling back to `ffmpeg`.
- First render downloads a Chrome Headless Shell build via
  `@remotion/renderer`'s browser management (cached under `HOME`, which is
  in the Rust-side `env_allow` list precisely so this cache is found on
  every subsequent render). Requires network access once; after that the
  cache is reused.
- `videoctl doctor --profile render` includes a `render.remotion_toolchain`
  check (Node executable + installed package) following the same
  honest-missing pattern as the existing `render.caption_renderer.listed`
  (libass) check — never reported `ok` unless actually verified.

## Licensing (checked 2026-08-02 for this pass)

Remotion's own license (the "Remotion License", not MIT) is free for:

- individuals, and
- companies/teams of **3 people or fewer**, regardless of revenue.

Damned Designs Studio / CutRight is a solo operation, so the free tier
applies. **Upgrade trigger: the moment a 4th person with any involvement in
this codebase or its commercial use joins** (employee, contractor, or
co-founder) — at that point a company license
(https://www.remotion.dev/license) must be purchased before continuing to
use Remotion, and this note must be updated to record the change. Re-verify
the license terms at https://remotion.dev/license before any headcount
change, not just once at integration time.

## Compositions

Canvas is fixed at 1280×720, 30fps, 45 frames (1.5s) — matching the
registry's previous ffmpeg-era motion-preview length so preview durations
did not change when the renderer did. Composition ids are the registry's
`effect_id` with `.` replaced by `-` (Remotion composition ids reject `.`):
`lower-third-identity-card-v1`, `stat-counter-v1`, `quote-card-v1`,
`cta-end-card-v1`.

## Determinism

Same props + same pinned Remotion version render byte-identical PNG frames.
`tests/render.test.ts::still renders are deterministic across two runs`
proves this by rendering the same composition+props twice and comparing a
SHA-256 of the output PNG bytes.
