#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const url = process.env.CUTRIGHT_QA_URL ?? readFileSync(".cache/qa-browser/url.txt", "utf8").trim();
const out = process.argv.includes("--out") ? process.argv[process.argv.indexOf("--out") + 1] : ".cache/qa-shots/studio.png";
const result = spawnSync(process.execPath, ["/Volumes/D/claude/tools/skills/qa/scripts/qa-shot.mjs", "--url", url, "--out", out], { stdio: "inherit" });
process.exit(result.status ?? 1);
