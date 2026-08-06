// CR-V2-B1-008 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/designer/engine (see skills/designer/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: it is never
// invoked as a bare shell command and performs no workspace mutation by itself.
// Execution happens only as the typed capability cutright://capability/designer.completion
// inside the signed runtime pack, emitting AssetDelivery records (no direct mutation).
// See skills/designer/CUTRIGHT-ADAPTATION.md for the capability mapping.

export function completionTypeForAcceptResult(eventType, acceptResult) {
  if (eventType === 'discard') return acceptResult?.handled === true ? 'discarded' : 'error';
  if (acceptResult?.handled === true && acceptResult?.carbonize === true) return 'agent_done';
  if (acceptResult?.handled === true) return 'complete';
  if (acceptResult?.mode === 'error') return 'error';
  if (eventType === 'accept' && acceptResult?.previewMode === 'svelte-component') return 'error';
  return 'agent_done';
}

export function completionAckForAcceptResult(eventId, completionType, acceptResult) {
  const ack = { ok: true, type: completionType };
  if (acceptResult?.handled === true && acceptResult?.carbonize === true) {
    ack.final = false;
    ack.requiresComplete = true;
    ack.nextCommand = `live-complete.mjs --id ${eventId}`;
    ack.message = 'Carbonize cleanup must be verified, then the session must be completed explicitly before polling again.';
  }
  return ack;
}
