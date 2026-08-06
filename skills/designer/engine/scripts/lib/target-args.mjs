// CR-V2-B1-008 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/designer/engine (see skills/designer/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: it is never
// invoked as a bare shell command and performs no workspace mutation by itself.
// Execution happens only as the typed capability cutright://capability/designer.target_args
// inside the signed runtime pack, emitting AssetDelivery records (no direct mutation).
// See skills/designer/CUTRIGHT-ADAPTATION.md for the capability mapping.

class TargetArgError extends Error {
  constructor(message, code) {
    super(message);
    this.name = 'TargetArgError';
    this.code = code;
  }
}

export function parseTargetPath(args = [], { strict = false } = {}) {
  let targetPath = null;
  for (let i = 0; i < args.length; i++) {
    const arg = String(args[i]);
    if (arg === '--target' || arg === '-t') {
      const next = args[i + 1];
      if (next && !String(next).startsWith('-')) {
        targetPath = String(next);
        i++;
        continue;
      }
      if (strict) {
        throw new TargetArgError('--target requires a path value.', 'TARGET_VALUE_MISSING');
      }
      continue;
    }
    if (arg.startsWith('--target=')) {
      const value = arg.slice('--target='.length);
      if (value) {
        targetPath = value;
        continue;
      }
      if (strict) {
        throw new TargetArgError('--target requires a path value.', 'TARGET_VALUE_MISSING');
      }
    }
  }
  return targetPath;
}

export function parseTargetOptions(args = [], options = {}) {
  const targetPath = parseTargetPath(args, options);
  return targetPath ? { targetPath } : {};
}
