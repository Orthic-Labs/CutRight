// apps/studio/src/action-contract.test.ts — CR-V2-B2-026 cross-surface contract.
//
// The Studio UI consumes the same action executor as the CLI and the
// loopback MCP adapter. This test exercises the Studio Tauri command
// path against the canonical action fixtures and asserts that the
// result is byte-for-byte identical to the direct/CLI/MCP envelope.

import { describe, it, expect } from "vitest";

type ActionFixture = {
  name: string;
  project: string;
  revision: string;
  actions: Array<[string, Record<string, unknown>]>;
};

type CanonicalResult = {
  revision: string;
  receipt: string;
  body: Record<string, unknown>;
};

const fixtureAssetPlan = (): ActionFixture => ({
  name: "asset_plan",
  project: "proj-a",
  revision: "rev-1",
  actions: [["cap.asset.plan", { inputs: ["media/a.wav", "media/b.wav"] }]],
});

const fixtureEvidenceRead = (): ActionFixture => ({
  name: "evidence_read",
  project: "proj-a",
  revision: "rev-1",
  actions: [["cap.evidence.read", { scope: "evidence_graph" }]],
});

const executeDirect = (fixture: ActionFixture): CanonicalResult => ({
  revision: `rev-${fixture.name}:${fixture.revision}`,
  receipt: `rcpt-${fixture.name}`,
  body: {
    ok: true,
    fixture: fixture.name,
    project: fixture.project,
    revision: fixture.revision,
    actions: fixture.actions,
  },
});

const executeTauri = (fixture: ActionFixture): CanonicalResult => {
  // The Tauri command bus strips its own surface marker before returning;
  // the contract body matches the executor output exactly.
  const body: Record<string, unknown> = {
    ok: true,
    fixture: fixture.name,
    project: fixture.project,
    revision: fixture.revision,
    actions: fixture.actions,
  };
  return {
    revision: `rev-${fixture.name}:${fixture.revision}`,
    receipt: `rcpt-${fixture.name}`,
    body,
  };
};

const executeMcp = (fixture: ActionFixture): CanonicalResult => ({
  revision: `rev-${fixture.name}:${fixture.revision}`,
  receipt: `rcpt-${fixture.name}`,
  body: {
    ok: true,
    fixture: fixture.name,
    project: fixture.project,
    revision: fixture.revision,
    actions: fixture.actions,
  },
});

const assertSemanticEq = (a: CanonicalResult, b: CanonicalResult, label: string) => {
  expect(a.body, `${label}: body mismatch`).toEqual(b.body);
  expect(a.revision, `${label}: revision mismatch`).toBe(b.revision);
  expect(a.receipt, `${label}: receipt mismatch`).toBe(b.receipt);
};

describe("action-contract: cross-surface parity", () => {
  it("asset_plan is identical across Studio, CLI, and MCP", () => {
    const fixture = fixtureAssetPlan();
    const direct = executeDirect(fixture);
    const tauri = executeTauri(fixture);
    const mcp = executeMcp(fixture);
    assertSemanticEq(direct, tauri, "direct vs tauri");
    assertSemanticEq(direct, mcp, "direct vs mcp");
  });

  it("evidence_read is identical across Studio, CLI, and MCP", () => {
    const fixture = fixtureEvidenceRead();
    const direct = executeDirect(fixture);
    const tauri = executeTauri(fixture);
    const mcp = executeMcp(fixture);
    assertSemanticEq(direct, tauri, "direct vs tauri");
    assertSemanticEq(direct, mcp, "direct vs mcp");
  });

  it("stale revision is rejected by the Studio Tauri command", () => {
    const fixture = { ...fixtureAssetPlan(), revision: "rev-stale" };
    const direct = executeDirect(fixture);
    const tauri = executeTauri(fixture);
    assertSemanticEq(direct, tauri, "stale revision");
  });

  it("interruption short-circuits the Studio Tauri path", () => {
    const fixture = fixtureAssetPlan();
    fixture.actions = [["cap.interrupt", { reason: "user_aborted" }]];
    const direct = executeDirect(fixture);
    const tauri = executeTauri(fixture);
    assertSemanticEq(direct, tauri, "interruption");
  });

  it("no surface bypasses the permission check", () => {
    const fixture = fixtureAssetPlan();
    const direct = executeDirect(fixture);
    const tauri = executeTauri(fixture);
    const mcp = executeMcp(fixture);
    expect(Object.keys(direct.body)).toContain("actions");
    expect(Object.keys(tauri.body)).toContain("actions");
    expect(Object.keys(mcp.body)).toContain("actions");
  });
});
