import { describe, expect, it } from "vitest";
import { DEFAULT_CLOUD_SETTINGS, MODE_LABEL, MODE_ORDER } from "./types";
import {
  isCloudSettingsValid,
  validateBudget,
  validateEnvVarName,
} from "./settings-logic";

describe("DEFAULT_CLOUD_SETTINGS", () => {
  it("has consent off by default", () => {
    expect(DEFAULT_CLOUD_SETTINGS.consent).toBe(false);
  });

  it("defaults to the proxy upload policy and no provider", () => {
    expect(DEFAULT_CLOUD_SETTINGS.upload_policy).toBe("proxy");
    expect(DEFAULT_CLOUD_SETTINGS.provider).toBe("disabled");
  });

  it("never carries a credential value out of the box", () => {
    expect(DEFAULT_CLOUD_SETTINGS.credential_env_var).toBeFalsy();
  });
});

describe("validateBudget", () => {
  it("accepts zero", () => {
    expect(validateBudget(0)).toBeNull();
  });

  it("accepts a positive amount", () => {
    expect(validateBudget(25.5)).toBeNull();
  });

  it("rejects a negative amount", () => {
    expect(validateBudget(-0.01)).toMatch(/negative/);
  });

  it("rejects NaN", () => {
    expect(validateBudget(Number.NaN)).toMatch(/number/);
  });

  it("rejects Infinity", () => {
    expect(validateBudget(Number.POSITIVE_INFINITY)).toMatch(/number/);
  });
});

describe("validateEnvVarName", () => {
  it("accepts an empty value (no credential configured yet)", () => {
    expect(validateEnvVarName("")).toBeNull();
  });

  it("accepts a real environment-variable name", () => {
    expect(validateEnvVarName("CUTRIGHT_GEMINI_KEY")).toBeNull();
  });

  it("accepts a name starting with an underscore", () => {
    expect(validateEnvVarName("_PRIVATE_KEY_NAME")).toBeNull();
  });

  it("trims surrounding whitespace before checking", () => {
    expect(validateEnvVarName("  CUTRIGHT_GEMINI_KEY  ")).toBeNull();
  });

  it("rejects a pasted API-key-shaped value, never storing it as a name", () => {
    const pastedKey = "AIzaSyD-fake-not-a-real-key-example12345";
    expect(validateEnvVarName(pastedKey)).toMatch(/not a credential value/);
  });

  it("rejects lowercase names", () => {
    expect(validateEnvVarName("cutright_gemini_key")).not.toBeNull();
  });

  it("rejects a name starting with a digit", () => {
    expect(validateEnvVarName("1KEY")).not.toBeNull();
  });

  it("rejects a name over the length limit", () => {
    expect(validateEnvVarName("A".repeat(129))).toMatch(/128/);
  });
});

describe("isCloudSettingsValid", () => {
  it("is valid for the defaults", () => {
    expect(isCloudSettingsValid(DEFAULT_CLOUD_SETTINGS)).toBe(true);
  });

  it("is invalid when the budget is negative", () => {
    expect(
      isCloudSettingsValid({ ...DEFAULT_CLOUD_SETTINGS, hard_budget_usd: -1 }),
    ).toBe(false);
  });

  it("is invalid when the credential field holds a key-shaped value", () => {
    expect(
      isCloudSettingsValid({
        ...DEFAULT_CLOUD_SETTINGS,
        credential_env_var: "sk-not-a-real-key-example",
      }),
    ).toBe(false);
  });
});

// Mode routing: the ⌘1-5 shortcuts (hooks/useKeyboard.ts), the command
// palette (components/CommandPalette.tsx), and the mode tabs (App.tsx) all
// derive from this one array/map instead of each hardcoding its own copy —
// this is what a mode-routing regression would actually break.
describe("MODE_ORDER / MODE_LABEL routing table", () => {
  it("has settings as the fifth mode, reachable via ⌘5", () => {
    expect(MODE_ORDER).toHaveLength(5);
    expect(MODE_ORDER[4]).toBe("settings");
  });

  it("lists every mode exactly once", () => {
    expect(new Set(MODE_ORDER).size).toBe(MODE_ORDER.length);
  });

  it("has a label for every mode in the order", () => {
    for (const mode of MODE_ORDER) {
      expect(MODE_LABEL[mode]).toBeTruthy();
    }
  });
});
