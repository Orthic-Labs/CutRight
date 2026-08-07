#!/usr/bin/env node
// CR-V2-B1-011 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/qa/scripts/qa-functional.mjs (see skills/qa/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: the base skill
// requires no Node installation on PATH and downloads nothing.
// Execution happens only as the typed capability cutright://capability/qa.qa_functional
// inside the signed qa runtime pack, driving an already-installed Chrome/Edge
// via headless flags and raw CDP, emitting evidence artifacts (no account access).
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const engine = join(here, "qa.mjs");
const args = process.argv.slice(2);
const finalArgs = args.includes("--actions") || args.some((a) => a.startsWith("--actions=")) ? args : ["--help"];
const result = spawnSync(process.execPath, [engine, ...finalArgs], { stdio: "inherit", shell: false });
process.exit(result.status ?? 1);
