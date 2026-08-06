#!/usr/bin/env node

// CR-V2-B1-008 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/designer/engine (see skills/designer/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: it is never
// invoked as a bare shell command and performs no workspace mutation by itself.
// Execution happens only as the typed capability cutright://capability/designer.live_discard_manual_edits
// inside the signed runtime pack, emitting AssetDelivery records (no direct mutation).
// See skills/designer/CUTRIGHT-ADAPTATION.md for the capability mapping.
/**
 * CLI helper: discard pending manual edits from the buffer without applying.
 *
 * Reads .impeccable/live/pending-manual-edits.json, drops entries, writes back.
 * No source-file writes. Use this when the user wants to throw away unsaved
 * manual edits.
 *
 * Trigger: only when the user explicitly asks the AI to discard / throw away /
 * clear pending manual edits.
 *
 * Usage:
 *   node live-discard-manual-edits.mjs              # discard all pending
 *   node live-discard-manual-edits.mjs --page-url=/ # discard only entries for "/"
 *
 * Output JSON: { discarded: N, entries: [...discardedEntries], totalCount: N }
 */

import { readBuffer, removeEntries, truncateBuffer } from './live/manual-edits-buffer.mjs';

function argVal(args, name) {
  const prefix = name + '=';
  for (const a of args) {
    if (a === name) return true;
    if (a.startsWith(prefix)) return a.slice(prefix.length);
  }
  return null;
}

const args = process.argv.slice(2);
if (args.includes('--help') || args.includes('-h')) {
  console.log('Usage: node live-discard-manual-edits.mjs [--page-url=<url>]');
  process.exit(0);
}

const pageUrlFilter = argVal(args, '--page-url');
const cwd = process.cwd();

let discarded;
let entries;
const buffer = readBuffer(cwd);
if (pageUrlFilter) {
  entries = buffer.entries.filter((entry) => entry.pageUrl === pageUrlFilter);
  discarded = removeEntries(cwd, (entry) => entry.pageUrl === pageUrlFilter);
} else {
  entries = buffer.entries;
  discarded = truncateBuffer(cwd);
}

const remaining = readBuffer(cwd).entries.reduce((n, e) => n + e.ops.length, 0);
console.log(JSON.stringify({ discarded, entries, totalCount: remaining }));
