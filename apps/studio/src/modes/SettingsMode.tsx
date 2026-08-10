import { useEffect, useState } from "react";
import { call } from "../lib/api";
import { CredentialField } from "../components/CredentialField";
import { SettingsDeleteConfirm } from "../components/SettingsDeleteConfirm";
import { AgentPanel } from "../components/AgentPanel";
import {
  isCloudSettingsValid,
  UPLOAD_POLICIES,
  validateBudget,
} from "../settings-logic";
import {
  DEFAULT_CLOUD_SETTINGS,
  type CloudSettings,
  type EngineStatus,
  type Snapshot,
  type UploadPolicy,
} from "../types";

const CAPABILITY_LABELS: Record<string, string> = {
  has_zscale: "HDR tone-map (zscale)",
  has_h264_videotoolbox: "Hardware preview (videotoolbox)",
  has_prores_ks: "ProRes master",
  has_lut3d: "Creative LUT",
  has_colortemperature: "White balance",
};

// The gap this mode fills (REV2 §15.6): cloud analysis is provider-agnostic
// and off by default, but until now there was nowhere in Studio for a user
// to consent, set a hard budget, choose an upload policy, or point at a
// credential once a provider ships. This surface gives that config
// somewhere to live now, without itself calling any provider — Phase 8's
// adapters are not built (STATUS.md), and `provider` can only be persisted
// as "disabled" until one exists (see `src-tauri/src/settings.rs`).
export function SettingsMode({ project }: { project: Snapshot }) {
  const [settings, setSettings] = useState<CloudSettings>(DEFAULT_CLOUD_SETTINGS);
  const [draft, setDraft] = useState<CloudSettings>(DEFAULT_CLOUD_SETTINGS);
  const [engine, setEngine] = useState<EngineStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);
  // Turning consent ON is spend-enabling (REV2 §15.6), so it gets its own
  // explicit two-step confirm in addition to the "Save changes" gate every
  // other field goes through — turning it back OFF is always safe and
  // needs neither. Reuses the segment-flag "arming" affordance already
  // established for compare-mode flagging (styles.css `.flag-segment`).
  const [arming, setArming] = useState(false);
  const [envPresent, setEnvPresent] = useState<boolean | null>(null);
  const [checkingEnv, setCheckingEnv] = useState(false);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [appInfo, setAppInfo] = useState<{ license: string; tier: string; offline: boolean; telemetry: boolean; updates: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError("");
    Promise.all([
      call<CloudSettings>("read_cloud_settings", { path: project.project_path }),
      call<EngineStatus>("read_engine_status"),
      call<typeof appInfo>("rightkit_app_info"),
    ])
      .then(([cloud, status, info]) => {
        if (cancelled) return;
        setSettings(cloud);
        setDraft(cloud);
        setEngine(status);
        setAppInfo(info);
      })
      .catch((reason) => {
        if (!cancelled) setError(String(reason));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [project.project_path]);

  const budgetError = validateBudget(draft.hard_budget_usd);
  const dirty =
    draft.consent !== settings.consent ||
    draft.hard_budget_usd !== settings.hard_budget_usd ||
    draft.upload_policy !== settings.upload_policy ||
    (draft.credential_env_var ?? "") !== (settings.credential_env_var ?? "");
  const canSave = dirty && isCloudSettingsValid(draft) && !saving;

  function toggleConsent() {
    if (draft.consent) {
      setDraft((current) => ({ ...current, consent: false }));
      setArming(false);
      return;
    }
    setArming(true);
  }

  function confirmEnable() {
    setDraft((current) => ({ ...current, consent: true }));
    setArming(false);
  }

  async function save() {
    setSaving(true);
    setError("");
    try {
      const persisted = await call<CloudSettings>("write_cloud_settings", {
        path: project.project_path,
        settings: draft,
      });
      setSettings(persisted);
      setDraft(persisted);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSaving(false);
    }
  }

  async function checkEnvVar() {
    const name = (draft.credential_env_var ?? "").trim();
    if (!name) return;
    setCheckingEnv(true);
    setError("");
    try {
      const present = await call<boolean>("credential_env_var_present", { name });
      setEnvPresent(present);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setCheckingEnv(false);
    }
  }

  async function confirmDelete() {
    setError("");
    try {
      const reset = await call<CloudSettings>("delete_cloud_data", {
        path: project.project_path,
      });
      setSettings(reset);
      setDraft(reset);
      setEnvPresent(null);
      setConfirmingDelete(false);
    } catch (reason) {
      setError(String(reason));
    }
  }

  if (loading) {
    return (
      <div className="settings-mode">
        <p className="settings-hint">Loading settings…</p>
      </div>
    );
  }

  return (
    <div className="settings-mode">
      <AgentPanel projectPath={project.project_path} />
      <section className="settings-section">
        <h2>Cloud analysis</h2>
        <p className="settings-hint">
          Off by default, and no project media leaves this machine until you
          explicitly enable it here. REV2 §15.6 cloud analysis (Gemini,
          Twelve Labs, …) is not built yet — this is where the consent,
          budget, upload policy, and credential a future provider adapter
          will read live once it ships.
        </p>

        <div className="settings-row">
          <div className="settings-row-copy">
            <b>Consent</b>
            <p>
              Allow this project to upload media to a cloud provider, once
              one is configured.
            </p>
          </div>
          <div className="settings-row-control">
            <button
              type="button"
              role="switch"
              aria-checked={draft.consent}
              className={`consent-toggle ${draft.consent ? "on" : ""}`}
              onClick={toggleConsent}
            >
              {draft.consent ? "Enabled" : "Disabled"}
            </button>
          </div>
        </div>
        {arming && (
          <div className="confirm-strip" role="status">
            <span>
              Enabling lets a future provider receive uploaded media for this
              project, up to the budget below, once you save.
            </span>
            <button className="flag-segment arming" onClick={confirmEnable}>
              Confirm enable
            </button>
            <button className="reason-cancel" onClick={() => setArming(false)}>
              Cancel
            </button>
          </div>
        )}

        <div className="settings-row">
          <div className="settings-row-copy">
            <label htmlFor="hard-budget">
              <b>Hard budget</b>
            </label>
            <p>Upper bound in US dollars this project&rsquo;s cloud analysis may spend.</p>
          </div>
          <div className="settings-row-control">
            <span className="budget-input-wrap">
              <span aria-hidden="true">$</span>
              <input
                id="hard-budget"
                type="number"
                min="0"
                step="0.01"
                inputMode="decimal"
                value={draft.hard_budget_usd}
                aria-invalid={Boolean(budgetError)}
                aria-describedby="hard-budget-error"
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    hard_budget_usd: event.target.valueAsNumber,
                  }))
                }
              />
            </span>
            <small id="hard-budget-error">
              {budgetError ? (
                <span className="bad">{budgetError}</span>
              ) : (
                "changes only take effect after Save changes below"
              )}
            </small>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <label htmlFor="upload-policy">
              <b>Upload policy</b>
            </label>
            <p>What media a future provider would receive.</p>
          </div>
          <div className="settings-row-control">
            <select
              id="upload-policy"
              value={draft.upload_policy}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  upload_policy: event.target.value as UploadPolicy,
                }))
              }
            >
              {UPLOAD_POLICIES.map((policy) => (
                <option key={policy.value} value={policy.value}>
                  {policy.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-row-copy">
            <label htmlFor="provider">
              <b>Provider</b>
            </label>
            <p>Gemini and Twelve Labs adapters are planned (§15.6) but not yet built.</p>
          </div>
          <div className="settings-row-control">
            <select id="provider" value="disabled" disabled>
              <option value="disabled">No provider configured</option>
            </select>
          </div>
        </div>

        <CredentialField
          value={draft.credential_env_var ?? ""}
          onChange={(value) => {
            setEnvPresent(null);
            setDraft((current) => ({
              ...current,
              credential_env_var: value.length ? value : null,
            }));
          }}
          present={envPresent}
          checking={checkingEnv}
          onCheck={checkEnvVar}
        />

        <div className="settings-actions">
          <button className="use-final" disabled={!canSave} onClick={save}>
            {saving ? "Saving…" : "Save changes"}
          </button>
          {saved && <span className="badge approved">✓ saved</span>}
        </div>
        {error && <p className="bad settings-error">{error}</p>}
      </section>

      <section className="settings-section" aria-label="CutRight legal and updates">
        <h2>CutRight</h2>
        <p className="settings-hint">{appInfo?.license ?? "Proprietary"} · {appInfo?.tier ?? "free"} · local/offline</p>
        <dl className="facts-grid"><dt>Telemetry</dt><dd>{appInfo?.telemetry === false ? "off" : "—"}</dd><dt>Updates</dt><dd>{appInfo?.updates ?? "—"}</dd></dl>
      </section>

      <section className="settings-section">
        <h2>Retention</h2>
        <p className="settings-hint">
          Resets cloud settings to defaults (consent off) and removes any
          cached cloud analysis data for this project.
        </p>
        <button className="reject" onClick={() => setConfirmingDelete(true)}>
          Delete cloud settings &amp; cache…
        </button>
      </section>

      <section className="settings-section">
        <h2>Engine &amp; project facts</h2>
        <dl className="facts-grid">
          <dt>Toolchain</dt>
          <dd>
            {engine?.resolved
              ? engine.toolchain_identity
              : `unresolved — ${engine?.error ?? "unknown error"}`}
          </dd>
          <dt>FFmpeg</dt>
          <dd>{engine?.ffmpeg_version ?? "—"}</dd>
          {engine?.capabilities && (
            <>
              <dt>Capabilities</dt>
              <dd className="capability-pills">
                {Object.entries(engine.capabilities)
                  .filter(([, enabled]) => enabled)
                  .map(([key]) => (
                    <span key={key} className="capability-pill">
                      {CAPABILITY_LABELS[key] ?? key}
                    </span>
                  ))}
                {Object.values(engine.capabilities).every((v) => !v) && (
                  <span className="capability-pill none">none detected</span>
                )}
              </dd>
            </>
          )}
          <dt>Project instance</dt>
          <dd>
            <code>{project.project_instance_id ?? "—"}</code>
          </dd>
          <dt>Project revision</dt>
          <dd>
            <code>{project.project_revision ?? "—"}</code>
          </dd>
        </dl>
        <p className="settings-hint">{engine?.note}</p>
      </section>

      {confirmingDelete && (
        <SettingsDeleteConfirm
          close={() => setConfirmingDelete(false)}
          onConfirm={confirmDelete}
        />
      )}
    </div>
  );
}
