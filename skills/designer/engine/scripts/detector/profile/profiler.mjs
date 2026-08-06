// CR-V2-B1-008 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/designer/engine (see skills/designer/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: it is never
// invoked as a bare shell command and performs no workspace mutation by itself.
// Execution happens only as the typed capability cutright://capability/designer.profiler
// inside the signed runtime pack, emitting AssetDelivery records (no direct mutation).
// See skills/designer/CUTRIGHT-ADAPTATION.md for the capability mapping.

function profileNow() {
  return typeof performance !== 'undefined' && performance.now
    ? performance.now()
    : Date.now();
}

function createDetectorProfile() {
  return { events: [] };
}

function recordProfileEvent(profile, event) {
  if (!profile) return;
  const normalized = {
    engine: event.engine || 'unknown',
    phase: event.phase || 'unknown',
    ruleId: event.ruleId || 'unknown',
    target: event.target || '',
    ms: Number.isFinite(event.ms) ? event.ms : 0,
    findings: Number.isFinite(event.findings) ? event.findings : 0,
  };
  if (event.detail) normalized.detail = event.detail;
  if (Array.isArray(event.findingIds) && event.findingIds.length) {
    normalized.findingIds = event.findingIds;
  }
  if (typeof profile === 'function') {
    profile(normalized);
  } else if (typeof profile.record === 'function') {
    profile.record(normalized);
  } else if (Array.isArray(profile.events)) {
    profile.events.push(normalized);
  } else if (Array.isArray(profile)) {
    profile.push(normalized);
  }
}

function extractFindingIds(findings) {
  if (!Array.isArray(findings) || findings.length === 0) return [];
  return [...new Set(findings.map(f => f?.id || f?.type || f?.antipattern).filter(Boolean))];
}

function profileFindings(profile, meta, callback) {
  if (!profile) return callback();
  const started = profileNow();
  const findings = callback();
  recordProfileEvent(profile, {
    ...meta,
    ms: profileNow() - started,
    findings: Array.isArray(findings) ? findings.length : 0,
    findingIds: extractFindingIds(findings),
  });
  return findings;
}

function profileStep(profile, meta, callback) {
  if (!profile) return callback();
  const started = profileNow();
  try {
    return callback();
  } finally {
    recordProfileEvent(profile, {
      ...meta,
      ms: profileNow() - started,
      findings: 0,
    });
  }
}

async function profileFindingsAsync(profile, meta, callback) {
  if (!profile) return callback();
  const started = profileNow();
  const findings = await callback();
  recordProfileEvent(profile, {
    ...meta,
    ms: profileNow() - started,
    findings: Array.isArray(findings) ? findings.length : 0,
    findingIds: extractFindingIds(findings),
  });
  return findings;
}

async function profileStepAsync(profile, meta, callback) {
  if (!profile) return callback();
  const started = profileNow();
  try {
    return await callback();
  } finally {
    recordProfileEvent(profile, {
      ...meta,
      ms: profileNow() - started,
      findings: 0,
    });
  }
}

function percentile(sortedValues, pct) {
  if (!sortedValues.length) return 0;
  const idx = Math.min(
    sortedValues.length - 1,
    Math.max(0, Math.ceil((pct / 100) * sortedValues.length) - 1),
  );
  return sortedValues[idx];
}

function summarizeDetectorProfile(profile) {
  const events = Array.isArray(profile)
    ? profile
    : (Array.isArray(profile?.events) ? profile.events : []);
  const groups = new Map();
  for (const event of events) {
    const key = [
      event.engine || 'unknown',
      event.phase || 'unknown',
      event.ruleId || 'unknown',
      event.target || '',
    ].join('\u0000');
    let group = groups.get(key);
    if (!group) {
      group = {
        engine: event.engine || 'unknown',
        phase: event.phase || 'unknown',
        ruleId: event.ruleId || 'unknown',
        target: event.target || '',
        calls: 0,
        totalMs: 0,
        findings: 0,
        samples: [],
      };
      groups.set(key, group);
    }
    const ms = Number.isFinite(event.ms) ? event.ms : 0;
    group.calls += 1;
    group.totalMs += ms;
    group.findings += Number.isFinite(event.findings) ? event.findings : 0;
    group.samples.push(ms);
  }
  return [...groups.values()]
    .map(group => {
      const samples = group.samples.sort((a, b) => a - b);
      return {
        engine: group.engine,
        phase: group.phase,
        ruleId: group.ruleId,
        target: group.target,
        calls: group.calls,
        totalMs: Number(group.totalMs.toFixed(3)),
        avgMs: Number((group.totalMs / group.calls).toFixed(3)),
        p50: Number(percentile(samples, 50).toFixed(3)),
        p95: Number(percentile(samples, 95).toFixed(3)),
        findings: group.findings,
      };
    })
    .sort((a, b) => b.totalMs - a.totalMs);
}

export {
  profileNow,
  createDetectorProfile,
  recordProfileEvent,
  extractFindingIds,
  profileFindings,
  profileStep,
  profileFindingsAsync,
  profileStepAsync,
  percentile,
  summarizeDetectorProfile,
};
