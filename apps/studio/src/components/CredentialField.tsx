import { validateEnvVarName } from "../settings-logic";

// Collects the NAME of an environment variable, never a credential value —
// this is the one control in the app that is deliberately incapable of
// holding a secret.
//
// Why: a credential pasted into a UI text field ends up in React state (so
// it survives in memory and in any state-inspector/devtools snapshot), in
// Tauri's IPC log if debug logging is on, in a crash report if one is
// generated while the field is populated, and in any screenshot or screen
// recording of this exact screen (including the ones this app's own
// support flow might ask a user for). None of those leakage paths are ones
// this component — or the operator, after the fact — can fully control.
// The only thing that is safe to store, log, and display is the NAME of an
// environment variable the operator sets in the engine's own process
// environment; `credential_env_var_present` reports only whether that
// variable is currently set (true/false), never its value, a prefix, or a
// masked preview. See `src-tauri/src/settings.rs` module docs for the
// backend half of this contract.
export function CredentialField({
  value,
  onChange,
  present,
  checking,
  onCheck,
}: {
  value: string;
  onChange: (value: string) => void;
  present: boolean | null;
  checking: boolean;
  onCheck: () => void;
}) {
  const error = validateEnvVarName(value);
  return (
    <div className="settings-row">
      <div className="settings-row-copy">
        <label htmlFor="cred-env-var">
          <b>Credential environment variable</b>
        </label>
        <p>
          e.g. <code>CUTRIGHT_GEMINI_KEY</code> — set this in the engine&rsquo;s
          own environment (shell profile, launch config). Studio never
          stores, echoes, logs, or transmits the key itself, only this name.
        </p>
      </div>
      <div className="settings-row-control credential-field">
        <input
          id="cred-env-var"
          type="text"
          inputMode="text"
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          placeholder="CUTRIGHT_GEMINI_KEY"
          value={value}
          onChange={(event) => onChange(event.target.value)}
          aria-invalid={Boolean(error)}
          aria-describedby="cred-env-var-note"
        />
        <button
          type="button"
          className="relink-btn"
          onClick={onCheck}
          disabled={checking || value.trim().length === 0 || Boolean(error)}
        >
          {checking ? "Checking…" : "Check"}
        </button>
        <small id="cred-env-var-note">
          {error ? (
            <span className="bad">{error}</span>
          ) : present === null ? (
            "presence is checked in the engine's environment — the value is never read"
          ) : present ? (
            <span className="good">✓ set in the engine&rsquo;s environment</span>
          ) : (
            <span className="warn">not set in the engine&rsquo;s environment</span>
          )}
        </small>
      </div>
    </div>
  );
}
