# Ingest workflow

1. Run `videoctl doctor`.
2. Create a project with `videoctl project init <folder>`.
3. Ingest sources through the typed CLI contract. The original files remain outside the project and
   are recorded by absolute path plus BLAKE3 hash.
4. Reject a source if its hash changes after registration.
