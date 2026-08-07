# CutRight v2 Project Index Architecture

## 1. Purpose

The Studio project index is a *disposable projection* of recent projects. It
is rebuilt from project packages and app-local history. Deleting the index
loses no project truth. Two projects with the same `title` remain distinct
because the index key is `project_instance_id`, not the title.

The authoritative source of project state is always the project package
itself (`<package_path>/project.json` and the rest of `PROJECT_DIRS`). The
index only lists and orders those packages for the Home mode.

## 2. Authoritative schema

`schemas/studio/project-index.schema.v1.json` defines the index shape. The
schema is closed (`additionalProperties: false`). The version is bumped only
when the index shape changes; rebuilding from existing packages does not
bump the schema version.

## 3. Row contract

```ts
type ProjectIndexRow = {
  project_instance_id: string;   // stable Studio-owned identity
  package_path: string;          // absolute path to the project package
  title: string;                 // user-visible title (may collide)
  lane: "recorded_footage" | "repurpose" | "explainer" | "anchored_creative";
  active_revision: string;       // current head revision
  run_status: "idle" | "running" | "ready" | "needs_review" | "failed" | "stale" | "missing";
  ready_count: number;
  needs_review_count: number;
  failed_count: number;
  updated_at: string;            // ISO 8601 monotonic per project
  thumbnail_hash?: string;
};
```

`run_status` is derived from jobs and digests. It is *not* a free-form
string. The seven values are exhaustive and stable; UI badges render from
this enum.

## 4. Operations

The index exposes the following operations. They are explicit, user-driven,
and do not silently mutate the underlying packages.

- **create**: open the new-project flow and produce a fresh package.
- **open**: navigate to the most recent mode for the project.
- **rename**: rename the *project title* in the project.json; the
  `project_instance_id` is immutable.
- **remove-from-list**: delete the index row only. Source media and the
  project package are untouched. A subsequent reimport can recreate the
  row from the package.

Source/project *deletion* (irreversible) is a separate explicit destructive
action and is never part of "remove-from-list".

## 5. Watch-folder import

`watch_folder_import_enabled` is `false` by default. When off, the index
only contains packages explicitly created or imported by the user. When on,
the OS-level folder watcher may add rows for newly observed packages. The
watcher is opt-in; it cannot delete or overwrite existing rows.

## 6. Rebuild / repair

`rebuild_index()` walks the registered package roots and the app-local
history of recent projects, then rewrites the index file. The function is
idempotent and never throws when a package path is missing: the missing row
is omitted and surfaced as a row with `run_status: "missing"` only when the
package was previously registered.

`repair_index(package_path)` looks up a single package by absolute path and
re-emits its row (or inserts a fresh one if not previously listed).

## 7. Status derivation

`run_status` is computed from the most recent job/digest state for the
project's `active_revision`. The derivation is deterministic and lives in
the Studio backend; the frontend never invents a status from free-form
strings.

| Backend state                                | `run_status`     |
| -------------------------------------------- | ---------------- |
| no jobs, ready digest                        | `ready`          |
| job running                                  | `running`        |
| digest reports `needs_review` items          | `needs_review`   |
| digest reports failed jobs                   | `failed`         |
| active_revision superseded                   | `stale`          |
| package path no longer resolves              | `missing`        |
| otherwise                                    | `idle`           |

## 8. Anti-promises

- The index is *not* a database of record. Deleting the index file loses
  no canonical project state.
- The index is *not* shared across machines. Each Studio instance rebuilds
  its own from the packages it can reach.
- The index does not record review or acceptance decisions. Those live in
  `feedback/decisions.jsonl` inside the project package.

## 9. Lane ownership

The Home mode (`apps/studio/src/modes/HomeMode.tsx`), the index hook
(`apps/studio/src/hooks/useProjectLibrary.ts`), and the index rebuild
backend (`apps/studio/src-tauri/src/project_index.rs`) are owned by Lane A
per `docs/dispatch/v2/book-6/interface-freeze.md`. No other lane may edit
the index file or the schema.