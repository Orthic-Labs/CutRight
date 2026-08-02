// Real Remotion render tests — no mocked bundler/renderer. These exercise
// the exact same `runPreview`/`bundleOnce` path scripts/render.mjs exposes
// to the Rust CLI boundary (crates/video-media/src/effects.rs
// EffectRenderer::Remotion), just invoked in-process instead of via a
// second `node` spawn.
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
// @ts-expect-error -- plain .mjs, no type declarations published for it.
import { runPreview } from "../scripts/render.mjs";
import {
  ctaEndCardSchema,
  lowerThirdIdentityCardSchema,
  quoteCardSchema,
  statCounterSchema,
} from "../src/schemas";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.join(__dirname, "..", "..", "..");
const FIXTURES = JSON.parse(
  fs.readFileSync(
    path.join(REPO_ROOT, "fixtures", "effects", "props-fixtures.json"),
    "utf8",
  ),
);

const REMOTION_EFFECTS = [
  { effectId: "lower-third.identity-card.v1", schema: lowerThirdIdentityCardSchema },
  { effectId: "stat-counter.v1", schema: statCounterSchema },
  { effectId: "quote-card.v1", schema: quoteCardSchema },
  { effectId: "cta-end-card.v1", schema: ctaEndCardSchema },
];

function tempDir(label: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), `cutright-effects-test-${label}-`));
  return dir;
}

function writePropsFile(dir: string, props: unknown): string {
  const file = path.join(dir, "props.json");
  fs.writeFileSync(file, JSON.stringify(props));
  return file;
}

function isPng(filePath: string): boolean {
  const bytes = fs.readFileSync(filePath);
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  return bytes.length > 0 && bytes.subarray(0, 8).equals(signature);
}

function sha256(filePath: string): string {
  return createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

describe("Remotion props schemas reject invalid props before any render", () => {
  it.each(REMOTION_EFFECTS)("$effectId", ({ effectId, schema }) => {
    const fixture = FIXTURES.effects[effectId];
    expect(schema.safeParse(fixture.valid).success).toBe(true);
    for (const invalid of fixture.invalid) {
      expect(schema.safeParse(invalid).success).toBe(false);
    }
  });
});

describe("each Remotion effect renders a real still preview", () => {
  it.each(REMOTION_EFFECTS)(
    "$effectId",
    async ({ effectId }) => {
      const dir = tempDir(effectId.replace(/\./g, "-"));
      const propsFile = writePropsFile(dir, FIXTURES.effects[effectId].valid);
      const outputs = await runPreview({
        composition: effectId,
        "props-file": propsFile,
        "output-dir": dir,
        motion: "false",
      });
      expect(fs.existsSync(outputs.still)).toBe(true);
      expect(isPng(outputs.still)).toBe(true);
      fs.rmSync(dir, { recursive: true, force: true });
    },
    120_000,
  );
});

describe("motion preview + reduced-motion variant both render", () => {
  it(
    "stat-counter.v1 renders still, motion, and motion-reduced",
    async () => {
      const effectId = "stat-counter.v1";
      const dir = tempDir("motion-stat-counter");
      const propsFile = writePropsFile(dir, FIXTURES.effects[effectId].valid);
      const outputs = await runPreview({
        composition: effectId,
        "props-file": propsFile,
        "output-dir": dir,
        motion: "true",
      });
      expect(fs.existsSync(outputs.still)).toBe(true);
      expect(fs.existsSync(outputs.motion)).toBe(true);
      expect(fs.existsSync(outputs.motionReduced)).toBe(true);
      expect(fs.statSync(outputs.motion).size).toBeGreaterThan(0);
      expect(fs.statSync(outputs.motionReduced).size).toBeGreaterThan(0);
      fs.rmSync(dir, { recursive: true, force: true });
    },
    180_000,
  );
});

describe("determinism: same props + same pinned version render byte-identical frames", () => {
  it(
    "cta-end-card.v1 still is byte-identical across two independent renders",
    async () => {
      const effectId = "cta-end-card.v1";
      const props = FIXTURES.effects[effectId].valid;

      const dirA = tempDir("determinism-a");
      const dirB = tempDir("determinism-b");
      const outputsA = await runPreview({
        composition: effectId,
        "props-file": writePropsFile(dirA, props),
        "output-dir": dirA,
        motion: "false",
      });
      const outputsB = await runPreview({
        composition: effectId,
        "props-file": writePropsFile(dirB, props),
        "output-dir": dirB,
        motion: "false",
      });

      expect(sha256(outputsA.still)).toBe(sha256(outputsB.still));

      fs.rmSync(dirA, { recursive: true, force: true });
      fs.rmSync(dirB, { recursive: true, force: true });
    },
    180_000,
  );
});
