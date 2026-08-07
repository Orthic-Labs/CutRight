# Privacy

The Studio never sends a byte off the device by default.

* Network access is **off** at install time and is a release policy,
  not a preference.
* Telemetry is **off** by default. The Privacy settings show a
  network-attempt audit counter; the counter is always zero in a
  default installation.
* Local logs are bounded and redacted. The default log line contains
  component, code, project pseudonymous id, revision, job/stage id,
  durations and hashes. Raw transcripts, source paths and API keys
  are never logged.
* Logs can be exported to a local file. The export always requires
  the operator to review the file list before it is written.
* Clearing diagnostics removes the log buffer. It never deletes
  canonical project evidence.

## Data on disk

* Project revisions are stored under the operator's chosen project
  root. Each revision is bound to a hash chain and a receipt tree.
* Pack bytes are stored under the operator's chosen pack root. The
  pack lock binds every byte to a signature.
* Decision records are stored under the project root and bound to
  the decision hash chain. They are never uploaded.
