// CR-V2-B1-008 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/designer/engine (see skills/designer/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: it is never
// invoked as a bare shell command and performs no workspace mutation by itself.
// Execution happens only as the typed capability cutright://capability/designer.live_target
// inside the signed runtime pack, emitting AssetDelivery records (no direct mutation).
// See skills/designer/CUTRIGHT-ADAPTATION.md for the capability mapping.

import path from 'node:path';
import { resolveProjectRoot } from './context.mjs';
import { parseTargetPath } from './lib/target-args.mjs';

export function resolveLiveTarget(cwd = process.cwd(), args = []) {
  const originalCwd = path.resolve(cwd);
  let targetPath = null;
  try {
    targetPath = parseTargetPath(args, { strict: true });
  } catch (err) {
    if (err?.name === 'TargetArgError') {
      process.stderr.write(`${err.message}\n`);
      process.exit(1);
    }
    throw err;
  }
  const absoluteTargetPath = targetPath
    ? path.isAbsolute(targetPath) ? targetPath : path.resolve(originalCwd, targetPath)
    : null;
  const projectRoot = targetPath
    ? resolveProjectRoot(originalCwd, { targetPath: absoluteTargetPath })
    : originalCwd;
  return {
    originalCwd,
    projectRoot,
    targetPath,
    absoluteTargetPath,
    targetOptions: absoluteTargetPath ? { targetPath: absoluteTargetPath } : {},
  };
}
