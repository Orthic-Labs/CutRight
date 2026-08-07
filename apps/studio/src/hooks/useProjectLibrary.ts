// apps/studio/src/hooks/useProjectLibrary.ts
//
// Book 6 task CR-V2-B6-007 — Lane A Home / rebuildable project library.
//
// React hook around the project index contract. Returns the current
// index, plus create / open / rename / remove-from-list / repair
// operations. Each operation surfaces its error through the standard
// `useProjectLibrary` API; the UI renders the typed error.
//
// The hook NEVER mutates canonical project state. The index is a
// disposable projection: deleting the file loses no project truth.

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  compareProjectIndexRows,
  isProjectIndex,
  PROJECT_INDEX_SCHEMA,
  type LaneId,
  type ProjectIndex,
  type ProjectIndexRow,
  type RunStatus,
} from "../contracts/projectIndex";

const TYPED_BRIDGE: { callBackend?: <T>(cmd: string, args?: unknown) => Promise<T> } = (
  (typeof window !== "undefined" && (window as any).__cutright_bridge) || {}
) as any;

async function callBackend<T>(cmd: string, args?: unknown): Promise<T> {
  if (TYPED_BRIDGE.callBackend) return TYPED_BRIDGE.callBackend<T>(cmd, args);
  throw new Error(
    `cutright studio backend bridge unavailable for ${cmd}; stub test environment.`,
  );
}

export type UseProjectLibraryResult = {
  ready: boolean;
  index: ProjectIndex | null;
  error: string | null;
  reload: () => Promise<void>;
  rebuild: () => Promise<void>;
  create: (input: {
    lane: LaneId;
    title: string;
    package_path: string;
  }) => Promise<ProjectIndexRow>;
  open: (project_instance_id: string) => Promise<void>;
  rename: (project_instance_id: string, title: string) => Promise<void>;
  removeFromList: (project_instance_id: string) => Promise<void>;
  repair: (package_path: string) => Promise<void>;
};

export function useProjectLibrary(): UseProjectLibraryResult {
  const [index, setIndex] = useState<ProjectIndex | null>(null);
  const [ready, setReady] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      const raw = await callBackend<unknown>("project_index.read");
      if (!isProjectIndex(raw)) {
        throw new Error(`project index has schema ${JSON.stringify(raw)}; expected ${PROJECT_INDEX_SCHEMA}`);
      }
      setIndex(raw);
      setError(null);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setReady(true);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const rebuild = useCallback(async () => {
    try {
      const raw = await callBackend<unknown>("project_index.rebuild");
      if (!isProjectIndex(raw)) {
        throw new Error("project_index.rebuild returned an invalid shape");
      }
      setIndex(raw);
      setError(null);
    } catch (e) {
      setError((e as Error).message);
    }
  }, []);

  const create = useCallback(
    async (input: { lane: LaneId; title: string; package_path: string }) => {
      try {
        const row = await callBackend<ProjectIndexRow>("project_index.create", input);
        setIndex((prev) => mergeRow(prev, row));
        setError(null);
        return row;
      } catch (e) {
        setError((e as Error).message);
        throw e;
      }
    },
    [],
  );

  const open = useCallback(async (project_instance_id: string) => {
    try {
      await callBackend<void>("project_index.open", { project_instance_id });
    } catch (e) {
      setError((e as Error).message);
      throw e;
    }
  }, []);

  const rename = useCallback(async (project_instance_id: string, title: string) => {
    try {
      const row = await callBackend<ProjectIndexRow>("project_index.rename", {
        project_instance_id,
        title,
      });
      setIndex((prev) => mergeRow(prev, row));
    } catch (e) {
      setError((e as Error).message);
      throw e;
    }
  }, []);

  const removeFromList = useCallback(async (project_instance_id: string) => {
    try {
      await callBackend<void>("project_index.remove_from_list", { project_instance_id });
      setIndex((prev) => stripRow(prev, project_instance_id));
    } catch (e) {
      setError((e as Error).message);
      throw e;
    }
  }, []);

  const repair = useCallback(async (package_path: string) => {
    try {
      await callBackend<void>("project_index.repair", { package_path });
      await reload();
    } catch (e) {
      setError((e as Error).message);
      throw e;
    }
  }, [reload]);

  return useMemo(
    () => ({ ready, index, error, reload, rebuild, create, open, rename, removeFromList, repair }),
    [ready, index, error, reload, rebuild, create, open, rename, removeFromList, repair],
  );
}

function mergeRow(prev: ProjectIndex | null, row: ProjectIndexRow): ProjectIndex {
  const base: ProjectIndex = prev ?? {
    schema: PROJECT_INDEX_SCHEMA,
    version: 1,
    rows: [],
    watch_folder_import_enabled: false,
  };
  const filtered = base.rows.filter((r) => r.project_instance_id !== row.project_instance_id);
  const rows = [row, ...filtered].sort(compareProjectIndexRows);
  return { ...base, rows };
}

function stripRow(prev: ProjectIndex | null, project_instance_id: string): ProjectIndex | null {
  if (!prev) return prev;
  return { ...prev, rows: prev.rows.filter((r) => r.project_instance_id !== project_instance_id) };
}

// Display helpers used by the Home grid. Derived from the enum so the UI
// never invents labels.
export function laneLabel(lane: LaneId): string {
  switch (lane) {
    case "recorded_footage":
      return "Recorded Footage";
    case "repurpose":
      return "Repurpose";
    case "explainer":
      return "Explainer";
    case "anchored_creative":
      return "Anchored Creative";
  }
}

export function statusAccent(status: RunStatus): "ok" | "warn" | "err" | "muted" {
  switch (status) {
    case "ready":
      return "ok";
    case "running":
      return "warn";
    case "needs_review":
    case "failed":
    case "stale":
    case "missing":
      return "err";
    case "idle":
      return "muted";
  }
}