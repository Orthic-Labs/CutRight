// CR-V2-B1-008 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/designer/engine (see skills/designer/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: it is never
// invoked as a bare shell command and performs no workspace mutation by itself.
// Execution happens only as the typed capability cutright://capability/designer.findings
// inside the signed runtime pack, emitting AssetDelivery records (no direct mutation).
// See skills/designer/CUTRIGHT-ADAPTATION.md for the capability mapping.

import { getAntipattern } from './registry/antipatterns.mjs';

function getAP(id) {
  return getAntipattern(id);
}

function finding(id, filePath, snippet, line = 0) {
  const ap = getAP(id);
  return { antipattern: id, name: ap.name, description: ap.description, severity: ap.severity || 'warning', file: filePath, line, snippet };
}

export { getAP, finding };
