#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// Resolve the shared QA runner portably instead of hard-coding one machine's
// volume layout. This file lives at <workspace>/apps/studio/scripts/, so the
// workspace root (which owns tools/skills/qa) is derived from import.meta.url.
// CUTRIGHT_QA_TOOLS overrides the tools root; a repo-local tools/ copy wins
// over the workspace-level one when present.
const scriptDir = dirname(fileURLToPath(import.meta.url));
const candidates = [
  process.env.CUTRIGHT_QA_TOOLS,
  join(scriptDir, "..", "..", "..", "tools"), // <cutright repo>/tools
  join(scriptDir, "..", "..", "..", "..", "tools"), // <workspace>/tools
]
  .filter(Boolean)
  .map((toolsRoot) => join(toolsRoot, "skills", "qa", "scripts", "qa-shot.mjs"));
const runner = candidates.find((candidate) => existsSync(candidate));
if (!runner) {
  console.error("[cutright-qa] shared QA runner not found. Tried:");
  for (const candidate of candidates) console.error(`[cutright-qa]   ${candidate}`);
  console.error(
    "[cutright-qa] set CUTRIGHT_QA_TOOLS to a tools root containing skills/qa/scripts/qa-shot.mjs",
  );
  process.exit(1);
}

const url = process.env.CUTRIGHT_QA_URL ?? readFileSync(".cache/qa-browser/url.txt", "utf8").trim();
const out = process.argv.includes("--out") ? process.argv[process.argv.indexOf("--out") + 1] : ".cache/qa-shots/studio.png";
const result = spawnSync(process.execPath, [runner, "--url", url, "--out", out], { stdio: "inherit" });
process.exit(result.status ?? 1);
