# CutRight v2 Security and Privacy Threat Model

This document is the canonical threat model for CutRight v2. It is the source
of truth for the `video-security` crate, the `video-project` trust status,
the `video-runtime` pack verifier, and the Studio trust panel. Anything not
in this document is out of scope for Book 7 Lane B.

## 1. Trust levels

Every external byte in the runtime is tagged with a **trust level**:

```text
immutable_source | canonical_project | verified_pack | imported_untrusted
| generated_untrusted | external_session
```

Boundaries:

- `immutable_source` — a media source registered with a BLAKE3 hash and never
  modified. Default for files produced by the local ingest pipeline.
- `canonical_project` — a project object inside the local store; revisions
  and actions are hash-chained.
- `verified_pack` — a runtime pack whose signature and content hashes match
  the active pack lock.
- `imported_untrusted` — anything imported from outside the local trust
  boundary: downloaded projects, third-party templates, legacy v1 exports,
  sample packs from elsewhere.
- `generated_untrusted` — output of a model, skill or helper that has not
  yet been validated and irreversibly filed.
- `external_session` — bytes supplied by an external MCP or session that is
  not part of the local trust boundary.

Every transition between trust levels is mediated by a validator and an
auditable event. **No external byte crosses a process boundary without a
validator and a sandbox grant.**

## 2. Threat catalogue

| Threat | Surface | Mitigation |
| --- | --- | --- |
| Malicious media | Ingest, decode, probe | Pre-decode validation against `media_limits`; sandboxed workers; typed unsupported on rejected input. |
| Crafted project package | Import, migration | Schema validation; revision ancestry check; user explicit confirmation. |
| Untrusted skill/model/pack | Catalogue, install | Hash + signature required; user-visible provenance; no implicit activation. |
| Path traversal | Project store, pack verifier | Canonical path resolution; allowlisted roots; reject `..` and absolute paths. |
| Decompression bomb | Pack, project, archive | Ratio cap before decompression; failure class `UnsupportedBomb`. |
| Process abuse | Worker spawn | Minimal env, scoped files, network denied, process tree control, time/output limits. |
| Prompt injection | Transcripts, model inputs | Skill/model inputs treated as `generated_untrusted`; never trusted for security decisions. |
| MCP misuse | External session | MCP grants recorded; not allowed to bypass trust level. |
| Tampered updates | Bundles, packs | Bundle manifest + signature; offline verification before activation. |
| Privacy leakage | Logs, telemetry, exports | Logs are local, bounded, redacted; telemetry off; audit counter for network attempts. |

## 3. Trust boundaries

The runtime draws a hard line between the local trust boundary and any
external byte. The boundary is enforced by:

- `WorkerGrant` — the only way a worker process can read or write files.
- `media_limits` — the only way a media/model artifact is admitted to a
  pipeline.
- `trust.rs` — the only way trust status is computed; never overridable.
- `recovery.rs` — never modifies source bytes.

A user, model, skill or MCP cannot extend, weaken or replace the boundary.

## 4. Data flows

```text
imported_untrusted → validator → sandboxed worker → generated_untrusted
                                                      ↓
                              canonisation → canonical_project (revision)
                                                      ↓
                                                render → immutable_source
```

Each `↓` is an auditable event using
`schemas/security/event.schema.v1.json`.

## 5. Secrets policy

- Release builds never read environment-variable overrides.
- No API key is required for core product operation.
- Credentials are read only through approved local signing interfaces
  (`scripts/release/v2-sign.py`) and never printed or inspected.
- Any debug or development credential is rejected by release builds.

## 6. Network policy

- Network access is **disabled** by default. This is enforced by the release
  policy, not a preference toggle.
- Workers are granted `network: Denied`. A worker that attempts to use the
  network is killed and recorded.
- A `network_attempt_total` counter is exposed for audit, even when no
  network is actually available.
- No URL field appears in user-facing offline v2 surfaces.

## 7. Filesystem scope

- Active project root is the only writable location outside staging.
- Workers may only read files in their grant list.
- Pack files are read-only and verified before use.
- The user's environment, `PATH`, sibling repositories, and global skill
  directories are **not** readable by workers or by release code.

## 8. Process sandbox

- Worker processes inherit minimal environment.
- Process tree control via `process_group`/`job_objects` where available.
- Time, output, temp, and resource limits are part of every `WorkerGrant`.
- A target that cannot meet the required sandbox guarantee returns a typed
  `Unsupported` and is **not** claimed supported.

## 9. Resource limits

- Container, stream and image dimensions, durations, counts, decompression
  ratios and metadata sizes are validated before any expensive decode.
- A failure is `Unsupported` for the call, not a worker crash.
- Quotas are per-pack and per-target; never silently expanded.

## 10. Audit events

Every security-relevant event is written using
`schemas/security/event.schema.v1.json`:

```json
{
  "schema_version": "v1",
  "event_id": "...",
  "kind": "trust_change | sandbox_deny | network_attempt | tamper | recovery_apply",
  "trust_from": "imported_untrusted",
  "trust_to": "generated_untrusted",
  "subject": "decision://...", "actor": "video-security",
  "at": "2026-08-07T00:00:00Z"
}
```

Audit events are local, never uploaded, and never deleted automatically.

## 11. Privacy

- Default logs contain: component, code, project pseudonymous id, revision,
  job/stage id, durations, hashes. Raw transcript, source paths and API keys
  are never logged.
- Diagnostics export is a user-reviewed file list.
- "Clear diagnostics" never deletes canonical project evidence without an
  explicit destructive confirmation.
- Telemetry is off. The network-attempt counter is local.

## 12. Safe recovery and degradation

- A failed decode degrades to a typed `Unsupported`; the user is informed.
- A tampered source is marked, not silenced.
- A missing pack disables the affected lane and surfaces target status.
- A privacy event produces a `PrivacyDiagnostic` the user can review and
  export.

## 13. Out of scope

- Hosted update publication and telemetry collection.
- Cloud fallbacks.
- Bare executable resolution from `PATH`.
- Reading a sibling repository or a global skill directory from release code.

## 14. Release policy

`config/security/release-policy.json` is the binding artefact. The policy
**enforces** network disabled; it is not a preference. The policy files are
required for release acceptance and absent optional scanners remain
unproven and may block according to release policy.
