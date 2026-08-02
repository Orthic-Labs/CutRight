import type { RelinkResult, SourceCheck } from "../contracts/review";

const shortHash = (value: string) =>
  value.length > 20 ? `${value.slice(0, 20)}…` : value;

// Renders the SOURCE INTEGRITY inspector rail. Named `SourceIntegrityPanel`
// (was `SourceIntegrity` in main.tsx, sharing that name with the
// `SourceIntegrity` grant type via TypeScript's separate type/value
// namespaces) since this module doesn't otherwise need the type/value split.
export function SourceIntegrityPanel({
  checks,
  verifying,
  progress,
  relinks,
  onVerify,
  onRelink,
}: {
  checks: SourceCheck[] | null;
  verifying: boolean;
  progress: string;
  relinks: Record<string, RelinkResult>;
  onVerify: () => void;
  onRelink: (sourceId: string) => void;
}) {
  const failed = checks?.filter((check) => !check.matches).length ?? 0;
  return (
    <div className="source-integrity">
      <div className="rail-head">
        <b>SOURCE INTEGRITY</b>
        {verifying && <span className="warn">{progress || "hashing…"}</span>}
      </div>
      <button className="verify-btn" onClick={onVerify} disabled={verifying}>
        {verifying ? `Verifying ${progress}` : "Verify sources"}
      </button>
      {checks && (
        <p className={`verify-summary ${failed ? "bad" : "good"}`}>
          {failed
            ? `${failed} of ${checks.length} sources failed verification`
            : `all ${checks.length} sources match the manifest`}
        </p>
      )}
      <ul className="verify-list">
        {(checks ?? []).map((check, index) => (
          <li
            key={check.source_id}
            className={`verify-row ${check.matches ? "pass" : "fail"}`}
            style={{ animationDelay: `${index * 45}ms` }}
          >
            <span className="verify-state" aria-hidden="true">
              {check.matches ? "✓" : "✕"}
            </span>
            <code className="verify-id">{check.source_id}</code>
            <code
              className="verify-hash"
              title={`expected ${check.expected_blake3}\nactual ${check.actual_blake3 ?? check.error ?? "unreadable"}`}
            >
              exp {shortHash(check.expected_blake3)}
              <br />
              act {shortHash(check.actual_blake3 ?? check.error ?? "missing")}
            </code>
            {!check.matches && (
              <button className="relink-btn" onClick={() => onRelink(check.source_id)}>
                Relink…
              </button>
            )}
            {relinks[check.source_id] && (
              <small className="relink-note">
                relinked · {shortHash(relinks[check.source_id].blake3)} ·{" "}
                {relinks[check.source_id].matches ? "match" : "still mismatched"}
              </small>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
