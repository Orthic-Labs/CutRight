// Tauri IPC dispatch (with a browser-QA in-memory mock) plus small display
// helpers shared across the app. Moved out of main.tsx per REV2 §14.4 — pure
// move, no behavior change.

import { convertFileSrc, invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  buildMockRecord,
  type DecisionIntent,
  type VariantSelection,
} from "../contracts/review";
import {
  fixture,
  fixtureWords,
  memoryDecisions,
  memoryMalformed,
  memorySelection,
  setMemorySelection,
} from "../fixtures/qa";

export const qa =
  new URLSearchParams(location.search).has("qa") ||
  import.meta.env.VITE_CUTRIGHT_QA === "1";

export async function call<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (!qa) return tauriInvoke<T>(command, args);
  if (command === "pick_project") return fixture.project_path as T;
  if (command === "read_transcript")
    return {
      words: fixtureWords[String(args.variant)] ?? fixtureWords.natural,
    } as T;
  if (command === "read_decisions")
    return {
      records: memoryDecisions,
      malformed_lines: memoryMalformed,
    } as T;
  if (command === "append_decision") {
    const intent = args.intent as DecisionIntent;
    const existing = memoryDecisions.find(
      (record) => record.client_request_id === intent.client_request_id,
    );
    if (existing) return existing as T;
    const record = buildMockRecord(
      intent,
      fixture.manifest.project_id ?? "qa-demo",
      memoryDecisions.length + 1,
    );
    memoryDecisions.push(record);
    return record as T;
  }
  if (command === "verify_sources")
    // source-a verifies clean; source-b drifted so QA can exercise the
    // mismatch banner and the relink flow.
    return fixture.sources.map((source, index) => ({
      source_id: source.source_id,
      path: `/QA/media/${source.source_id}.mp4`,
      expected_blake3: `blake3:expected-${source.source_id}`,
      actual_blake3:
        index === 1
          ? "blake3:drifted-bytes"
          : `blake3:expected-${source.source_id}`,
      matches: index === 0,
      error: null,
    })) as T;
  if (command === "relink_source") {
    const sourceId = String(args.source_id);
    return {
      source_id: sourceId,
      path: String(args.new_path),
      blake3: `blake3:expected-${sourceId}`,
      matches: true,
    } as T;
  }
  if (command === "select_variant") {
    const variant = String(args.variant);
    const next: VariantSelection = {
      schema_version: 1,
      variant,
      rough_cut_path: `render/rough-cuts/${variant}.mp4`,
      rough_cut_blake3: `blake3:mock-${variant}-roughcut`,
      rough_cut_size: 12345678,
      selected_at: new Date().toISOString(),
      selected_by: "qa-mock",
    };
    setMemorySelection(next);
    return next as T;
  }
  if (command === "read_variant_selection") return memorySelection as T;
  throw new Error(`QA mock has no ${command}`);
}
export const tc = (value = 0) => {
  const s = Math.max(0, Math.floor(value / 1000));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
};
export const asset = (path?: string | null) =>
  path ? (qa ? path : convertFileSrc(path)) : undefined;
