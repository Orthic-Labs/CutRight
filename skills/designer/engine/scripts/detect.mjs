#!/usr/bin/env node

// CR-V2-B1-008 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/designer/engine (see skills/designer/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: it is never
// invoked as a bare shell command and performs no workspace mutation by itself.
// Execution happens only as the typed capability cutright://capability/designer.detect
// inside the signed runtime pack, emitting AssetDelivery records (no direct mutation).
// See skills/designer/CUTRIGHT-ADAPTATION.md for the capability mapping.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const candidates = [
  path.join(__dirname, 'detector', 'detect-antipatterns.mjs'),
  path.join(__dirname, '..', '..', 'cli', 'engine', 'detect-antipatterns.mjs'),
];
const detectorPath = candidates.find(p => fs.existsSync(p));

if (!detectorPath) {
  process.stderr.write('Error: bundled detector not found.\n');
  process.exit(1);
}

const { detectCli } = await import(pathToFileURL(detectorPath));

await detectCli();
