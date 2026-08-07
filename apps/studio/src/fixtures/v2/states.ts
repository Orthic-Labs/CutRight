// apps/studio/src/fixtures/v2/states.ts — CR-V2-B6-024.
export const FIXTURE_ID = "fixture-v2-stable";
export const FROZEN_DATE = "2026-08-07T00:00:00Z";
export const V2_FIXTURE_STATES = ["normal","empty","loading","degraded","needs-review","failure","stale","corrupt"] as const;
export type V2FixtureState = (typeof V2_FIXTURE_STATES)[number];
