import { useState, useMemo } from "react";
import type {
  DecisionReason,
  DecisionAxis,
} from "../contracts/feedback";

export interface FormatProfileValues {
  inherited_defaults: Record<string, string>;
  overrides: Record<string, string>;
}

export interface FormatProfile {
  schema_version: "v1";
  profile_id: string;
  format: { content_type: string; platform: string; variant: string };
  version: string;
  compatibility: {
    pack_set_id: string;
    pack_set_fingerprint: string;
    benchmark_profile: string;
    skill_version: string;
    render_version: string;
  };
  values: FormatProfileValues;
  source_recommendation_hash: string;
  source_decision_ids: string[];
  approved_by: "user_reviewed" | "user_rejected" | "user_replaced" | "user_noted" | "system";
  approved_at: string;
}

/**
 * FormatProfilePanel — read-only display of a FormatProfile. The user can
 * inspect the source decision IDs for each setting. The note field is
 * opt-in; the user is never forced to add a note.
 *
 * Profiles are immutable. To change a setting, the user creates a new
 * version explicitly via the approve action.
 */
export function FormatProfilePanel({ profile }: { profile: FormatProfile }) {
  const [expanded, setExpanded] = useState(false);
  const sortedOverrides = useMemo(
    () => Object.entries(profile.values.overrides).sort((a, b) => a[0].localeCompare(b[0])),
    [profile],
  );
  return (
    <section data-testid="format-profile-panel">
      <header>
        <h2>
          {profile.format.content_type} / {profile.format.platform} / {profile.format.variant}
        </h2>
        <span>v{profile.version}</span>
      </header>
      <p>approved_by: {profile.approved_by}</p>
      <p>approved_at: {profile.approved_at}</p>
      <button type="button" onClick={() => setExpanded((v) => !v)}>
        {expanded ? "Hide" : "Show"} {sortedOverrides.length} setting(s)
      </button>
      {expanded && (
        <ul>
          {sortedOverrides.map(([key, value]) => (
            <li key={key}>
              <strong>{key}</strong>: {value}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export default FormatProfilePanel;
