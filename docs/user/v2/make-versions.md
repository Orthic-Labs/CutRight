# Make Versions

`Make Versions` is the primary production surface. It produces one
output per format key (`content_type × platform × variant`).

1. Select the source project.
2. Choose the format set. By default Studio offers the format keys
   that match the active pack set and the project history.
3. Studio plans, renders and reviews each version. A version lands
   in `ready` when the independent critic and the deterministic QA
   floors pass; otherwise it lands in `needs_review` or `failed`.
4. The last approved final is preserved. The new version is staged
   alongside it; you accept, note, or reject the new version
   explicitly.

## Versioning

* Every version is an immutable v2 revision. Edits create new
  revisions, never overwrite the prior one.
* A version is bound to the exact pack set, app version and review
  mode. Changing any of these forces a new version.
* The `autonomous` review mode is only available for formats whose
  benchmark floor has been met. Studio shows the floor status before
  the run starts.
