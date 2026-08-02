#!/usr/bin/env node
// The one CLI entry point the Rust renderer (crates/video-media/src/effects.rs
// EffectRenderer::Remotion branch) shells out to, always through
// video_core::process_runner (bounded timeout, env allow-list, capped
// output) — never a bare spawn. See ../README.md for the full contract.
//
// Commands:
//   bundle                                  — webpack-bundle only (gate.sh build step; no browser needed)
//   probe [--composition <effect-id>]       — bundle + resolve one composition (doctor / toolchain check; no frame render)
//   preview --composition <effect-id> --props-file <path> --output-dir <dir>
//           --motion <true|false> [--duration <secs>] [--width <n>] [--height <n>]
//                                            — real render: always still.png; when --motion true, also
//                                              motion.mp4 + motion-reduced.mp4
//
// Exits non-zero with a message on stderr for every failure mode — never
// silently produces a partial or fake output.

import { bundle } from "@remotion/bundler";
import { renderMedia, renderStill, selectComposition } from "@remotion/renderer";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PACKAGE_ROOT = path.join(__dirname, "..");
const ENTRY_POINT = path.join(PACKAGE_ROOT, "src", "index.ts");

const DEFAULT_FPS = 30;
const DEFAULT_WIDTH = 1280;
const DEFAULT_HEIGHT = 720;
const DEFAULT_DURATION_SECS = 1.5;

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i++) {
    const token = argv[i];
    if (!token.startsWith("--")) continue;
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next !== undefined && !next.startsWith("--")) {
      args[key] = next;
      i++;
    } else {
      args[key] = "true";
    }
  }
  return args;
}

/** Registry effect_id -> Remotion composition id. Mirrors
 * src/schemas.ts::EFFECT_ID_TO_COMPOSITION_ID mechanically (dots aren't
 * legal in Remotion composition ids) without requiring a TS build step to
 * run this plain-Node script. */
function effectIdToCompositionId(effectId) {
  return effectId.replace(/\./g, "-");
}

async function bundleOnce() {
  return bundle({
    entryPoint: ENTRY_POINT,
    onProgress: () => {},
  });
}

async function runBundle() {
  const serveUrl = await bundleOnce();
  process.stdout.write(JSON.stringify({ ok: true, command: "bundle", serveUrl }) + "\n");
}

async function runProbe(args) {
  const effectId = args.composition ?? "cta-end-card.v1";
  const compositionId = effectIdToCompositionId(effectId);
  const serveUrl = await bundleOnce();
  const composition = await selectComposition({
    serveUrl,
    id: compositionId,
    inputProps: {},
  });
  process.stdout.write(
    JSON.stringify({
      ok: true,
      command: "probe",
      compositionId: composition.id,
      width: composition.width,
      height: composition.height,
      fps: composition.fps,
      durationInFrames: composition.durationInFrames,
    }) + "\n",
  );
}

async function runPreview(args) {
  const effectId = args.composition;
  if (!effectId) throw new Error("preview requires --composition <effect-id>");
  const propsFile = args["props-file"];
  if (!propsFile) throw new Error("preview requires --props-file <path>");
  const outputDir = args["output-dir"];
  if (!outputDir) throw new Error("preview requires --output-dir <dir>");

  const compositionId = effectIdToCompositionId(effectId);
  const props = JSON.parse(fs.readFileSync(propsFile, "utf8"));
  const wantsMotion = args.motion === "true";
  const width = parseInt(args.width ?? String(DEFAULT_WIDTH), 10);
  const height = parseInt(args.height ?? String(DEFAULT_HEIGHT), 10);
  const fps = DEFAULT_FPS;
  const durationSecs = parseFloat(args.duration ?? String(DEFAULT_DURATION_SECS));
  const durationInFrames = Math.max(1, Math.round(durationSecs * fps));

  fs.mkdirSync(outputDir, { recursive: true });

  const serveUrl = await bundleOnce();
  const composition = await selectComposition({
    serveUrl,
    id: compositionId,
    inputProps: props,
  });
  const resolved = {
    ...composition,
    width,
    height,
    fps,
    durationInFrames,
  };

  // Still: the fully settled state (last frame), rendered with the
  // animated (non-reduced-motion) prop set — this mirrors the previous
  // ffmpeg-era still preview, which always showed the finished composite.
  const stillPath = path.join(outputDir, "still.png");
  await renderStill({
    composition: resolved,
    serveUrl,
    output: stillPath,
    inputProps: { ...props, reducedMotion: false },
    frame: durationInFrames - 1,
  });

  const outputs = { still: stillPath };

  if (wantsMotion) {
    const motionPath = path.join(outputDir, "motion.mp4");
    await renderMedia({
      composition: resolved,
      serveUrl,
      codec: "h264",
      outputLocation: motionPath,
      inputProps: { ...props, reducedMotion: false },
    });

    const reducedPath = path.join(outputDir, "motion-reduced.mp4");
    await renderMedia({
      composition: resolved,
      serveUrl,
      codec: "h264",
      outputLocation: reducedPath,
      inputProps: { ...props, reducedMotion: true },
    });

    outputs.motion = motionPath;
    outputs.motionReduced = reducedPath;
  }

  process.stdout.write(JSON.stringify({ ok: true, command: "preview", outputs }) + "\n");
  return outputs;
}

async function main() {
  const [command, ...rest] = process.argv.slice(2);
  const args = parseArgs(rest);

  switch (command) {
    case "bundle":
      return runBundle();
    case "probe":
      return runProbe(args);
    case "preview":
      return runPreview(args);
    default:
      throw new Error(
        `unknown command ${JSON.stringify(command)}; expected one of: bundle, probe, preview`,
      );
  }
}

// Exported so tests/render.test.ts can drive real renders directly without
// shelling out to a second Node process per case — same render path the CLI
// uses (`runPreview`/`bundleOnce`), just invoked in-process.
export { bundleOnce, effectIdToCompositionId, runPreview, runProbe };

// Only run as a CLI when invoked directly (`node scripts/render.mjs ...`),
// not when imported by the test suite.
const isMain = process.argv[1] && import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  main().catch((error) => {
    process.stderr.write(`${error && error.stack ? error.stack : String(error)}\n`);
    process.exitCode = 1;
  });
}
