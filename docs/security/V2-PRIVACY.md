# V2 Privacy Defaults

The v2 release ships with privacy and telemetry defaults that are
**enforced by code**, not by user preference. Reviewers can prove the
defaults through the integration tests in `crates/video-security/tests/privacy.rs`
and through the Tauri-side `PrivacySettings::default()` in
`apps/studio/src-tauri/src/privacy_settings.rs`.

## Defaults

| Surface                      | Default           | Why                                            |
| ---------------------------- | ----------------- | ---------------------------------------------- |
| Telemetry                    | Off               | The v2 release has no telemetry transport.      |
| Network                      | Blocked           | The worker sandbox forces `NetworkPolicy::Denied`.|
| Raw transcript in logs       | Redacted          | `redact()` strips `"transcript:"`, `"prompt:"`, `"apikey"`. |
| Raw prompt in logs           | Redacted          | Same as above.                                 |
| API key in logs               | Redacted          | Same as above.                                  |
| Project identifier in logs   | Pseudonymous      | Blaked `project_pseudonym_salt` + raw id.      |
| Diagnostics clear            | Canonical kept    | `clear_diagnostics()` returns a `ClearReport` with `canonical_untouched = true`. |

## Network-attempt audit

Network attempts are recorded via `network_attempt_record()` even when the
release policy denies them. This makes the blocked-network target auditable:
the count is visible from `PrivacySettings::network_attempts.attempts`.

## PrivacySettings location

The Studio Tauri side persists these in `PrivacySettings`, defaulting to:

```rust
telemetry_enabled: false,
network_allowed: false,
raw_transcript_export_allowed: false,
raw_prompt_export_allowed: false,
```

There is no `enable_telemetry()` API. To honour the v2 release policy, an
external contributor would need to add an opt-in field — outside the scope
of this book.
