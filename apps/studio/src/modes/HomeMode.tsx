// apps/studio/src/modes/HomeMode.tsx
//
// Book 6 task CR-V2-B6-007 — Lane A Home / rebuildable project library.
//
// The Home mode renders the rebuildable project library. It shows recent
// projects as cards (see `ProjectCard`), supports search / filter, and
// exposes primary actions for the four production lanes (Recorded Footage,
// Repurpose, Explainer, Anchored Creative). The index is a disposable
// projection; deleting it loses no project truth.

import { useMemo, useState } from "react";
import { ProjectCard } from "../components/ProjectCard";
import { useProjectLibrary } from "../hooks/useProjectLibrary";
import type { LaneId } from "../contracts/projectIndex";

const PRIMARY_LANES: readonly LaneId[] = [
  "recorded_footage",
  "repurpose",
  "explainer",
  "anchored_creative",
];

export function HomeMode(props: {
  onOpenProject: (project_instance_id: string) => void;
  onCreateProject: (lane: LaneId) => void;
}) {
  const library = useProjectLibrary();
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "needs_review" | "failed">("all");

  const rows = useMemo(() => {
    if (!library.index) return [];
    const q = query.trim().toLowerCase();
    return library.index.rows.filter((row) => {
      if (q.length > 0 && !row.title.toLowerCase().includes(q)) return false;
      if (filter === "needs_review" && row.needs_review_count === 0) return false;
      if (filter === "failed" && row.failed_count === 0) return false;
      return true;
    });
  }, [library.index, query, filter]);

  return (
    <main className="home-mode" aria-label="Project library">
      <header>
        <h1>Projects</h1>
        <p className="hint">
          The library is a rebuildable index. Deleting the index file loses no project truth; rebuild from
          the package directory at any time.
        </p>
        {library.error && (
          <p role="alert" className="library-error">
            {library.error}
          </p>
        )}
      </header>

      <section className="primary-actions" aria-label="Create project">
        <h2>Start a new project</h2>
        <ul>
          {PRIMARY_LANES.map((lane) => (
            <li key={lane}>
              <button onClick={() => props.onCreateProject(lane)} data-lane={lane}>
                {lane === "recorded_footage" && "Recorded Footage"}
                {lane === "repurpose" && "Repurpose"}
                {lane === "explainer" && "Explainer"}
                {lane === "anchored_creative" && "Anchored Creative"}
              </button>
            </li>
          ))}
        </ul>
      </section>

      <section className="library-controls" aria-label="Filter">
        <label>
          Search
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            placeholder="Filter by title"
          />
        </label>
        <fieldset>
          <legend>Show</legend>
          <label>
            <input
              type="radio"
              name="filter"
              value="all"
              checked={filter === "all"}
              onChange={() => setFilter("all")}
            />
            All
          </label>
          <label>
            <input
              type="radio"
              name="filter"
              value="needs_review"
              checked={filter === "needs_review"}
              onChange={() => setFilter("needs_review")}
            />
            Needs review
          </label>
          <label>
            <input
              type="radio"
              name="filter"
              value="failed"
              checked={filter === "failed"}
              onChange={() => setFilter("failed")}
            />
            Failed
          </label>
        </fieldset>
        <button onClick={() => void library.reload()} aria-label="Reload index">
          Reload
        </button>
        <button onClick={() => void library.rebuild()} aria-label="Rebuild index">
          Rebuild index
        </button>
      </section>

      <section className="library-grid" aria-label="Recent projects">
        {library.ready && rows.length === 0 && <p>No projects yet.</p>}
        {rows.map((row) => (
          <ProjectCard
            key={row.project_instance_id}
            row={row}
            onOpen={(id) => props.onOpenProject(id)}
            onRename={(id, title) => {
              const next = window.prompt("Rename project", title);
              if (next && next.trim().length > 0) void library.rename(id, next.trim());
            }}
            onRemoveFromList={(id) => void library.removeFromList(id)}
          />
        ))}
      </section>
    </main>
  );
}