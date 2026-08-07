// apps/studio/src/components/ProjectCard.tsx
//
// Book 6 task CR-V2-B6-007 — Lane A Home / rebuildable project library.
//
// Renders a single project card in the Home grid. Status, ready, review
// and failed counts come from the disposable project-index row (not from
// free-form strings). Actions are wired through the parent `useProjectLibrary`.

import type { ProjectIndexRow } from "../contracts/projectIndex";
import { runStatusLabel } from "../contracts/projectIndex";
import { laneLabel, statusAccent } from "../hooks/useProjectLibrary";

export function ProjectCard(props: {
  row: ProjectIndexRow;
  onOpen: (id: string) => void;
  onRename: (id: string, currentTitle: string) => void;
  onRemoveFromList: (id: string) => void;
}) {
  const { row, onOpen, onRename, onRemoveFromList } = props;
  const accent = statusAccent(row.run_status);
  const counts = `${row.ready_count} ready · ${row.needs_review_count} review · ${row.failed_count} failed`;

  return (
    <article className={`project-card accent-${accent}`} data-testid={`project-card-${row.project_instance_id}`}>
      <header>
        <span className="lane-badge" data-lane={row.lane}>
          {laneLabel(row.lane)}
        </span>
        <span className={`run-badge run-${row.run_status}`}>{runStatusLabel(row.run_status)}</span>
      </header>
      <h3 className="title">{row.title}</h3>
      <p className="counts" aria-label="digest counts">
        {counts}
      </p>
      <footer>
        <button onClick={() => onOpen(row.project_instance_id)} aria-label={`Open ${row.title}`}>
          Open
        </button>
        <button
          onClick={() => onRename(row.project_instance_id, row.title)}
          aria-label={`Rename ${row.title}`}
        >
          Rename
        </button>
        <button
          onClick={() => onRemoveFromList(row.project_instance_id)}
          aria-label={`Remove ${row.title} from library list`}
        >
          Remove from list
        </button>
      </footer>
    </article>
  );
}