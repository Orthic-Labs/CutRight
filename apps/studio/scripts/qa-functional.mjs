#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const url = process.env.CUTRIGHT_QA_URL ?? readFileSync(".cache/qa-browser/url.txt", "utf8").trim();
const actions = process.argv.includes("--actions") ? process.argv[process.argv.indexOf("--actions") + 1] : ".cache/qa-actions.json";
const result = spawnSync(process.execPath, ["/Volumes/D/claude/tools/skills/qa/scripts/qa-functional.mjs", "--url", url, "--actions", actions], { stdio: "inherit" });
process.exit(result.status ?? 1);
