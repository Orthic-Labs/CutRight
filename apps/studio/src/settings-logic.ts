// Pure validation/shaping helpers for SettingsMode, pulled out of the
// component so they're unit-testable without mounting React (this repo has
// no component-render test harness — see hooks/useFocusTrap.test.ts and
// friends for the established pattern of testing the pure logic a
// component drives, not the component itself).

import type { CloudSettings, UploadPolicy } from "./types";

/// Mirrors the backend's `CloudSettings::validate` budget rule
/// (`src-tauri/src/settings.rs`) so the Save button can disable itself and
/// show an inline error before round-tripping to Rust at all.
export function validateBudget(value: number): string | null {
  if (!Number.isFinite(value)) return "budget must be a number";
  if (value < 0) return "budget cannot be negative";
  return null;
}

// An environment-variable NAME is short, upper-cased, and made of
// [A-Z0-9_] starting with a letter or underscore. This deliberately mirrors
// the backend's `credential_env_var` shape check
// (`src-tauri/src/settings.rs::CloudSettings::validate`) on the frontend
// too, so a pasted API key value (mixed case, punctuation, often 30+ chars)
// is rejected at the input before it is ever sent anywhere — belt-and-
// braces around the same rule, not a second source of truth for it.
const ENV_VAR_NAME_RE = /^[A-Z_][A-Z0-9_]*$/;

export function validateEnvVarName(raw: string): string | null {
  const name = raw.trim();
  if (name.length === 0) return null; // empty is valid: "no credential set"
  if (name.length > 128) return "must be 128 characters or fewer";
  if (!ENV_VAR_NAME_RE.test(name))
    return "must look like an environment variable name (A-Z, 0-9, _) — not a credential value";
  return null;
}

export function isCloudSettingsValid(settings: CloudSettings): boolean {
  return (
    validateBudget(settings.hard_budget_usd) === null &&
    validateEnvVarName(settings.credential_env_var ?? "") === null
  );
}

export const UPLOAD_POLICIES: Array<{ value: UploadPolicy; label: string }> = [
  { value: "proxy", label: "Proxy (recommended) — downscaled copy only" },
  { value: "source", label: "Source — original media, uses more budget/bandwidth" },
];
