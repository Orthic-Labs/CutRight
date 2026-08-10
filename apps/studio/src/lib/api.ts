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
  memoryCloudSettings,
  memoryDecisions,
  memoryMalformed,
  memorySelection,
  setMemoryCloudSettings,
  setMemorySelection,
} from "../fixtures/qa";
import { DEFAULT_CLOUD_SETTINGS, type CloudSettings } from "../types";

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
  if (command === "rightkit_app_info") return { schema_version: 1, app: "cutright", tier: "free", license: "Proprietary", offline: true, telemetry: false, updates: "disabled-until-configured" } as T;
  if (command === "rightkit_logs_write" || command === "rightkit_logs_clear") return undefined as T;
  if (command === "rightkit_logs_collect") return [] as T;
  if (command === "finish_read_variants") return {
    variants: ["balanced", "pullback", "punch", "push", "editor-takeover"].map((id, index) => ({
      id, label: id.replace("-", " "), preview_url: null, source_hashes: [`blake3:qa-${id}`], score: 1 - index * 0.05,
    })),
  } as T;
  if (command === "finish_commit_variant") return {
    variantId: String(args.variant_id),
    lockedCutHash: String(args.locked_cut_hash),
    sourceHashes: Array.isArray(args.source_hashes) ? args.source_hashes : [],
  } as T;
  if (command === "read_cloud_settings") return memoryCloudSettings as T;
  if (command === "write_cloud_settings") {
    const next = {
      ...(args.settings as CloudSettings),
      updated_at: new Date().toISOString(),
    };
    if (next.hard_budget_usd < 0 || !Number.isFinite(next.hard_budget_usd))
      throw new Error("hard_budget_usd: must be zero or a positive number");
    setMemoryCloudSettings(next);
    return next as T;
  }
  if (command === "delete_cloud_data") {
    setMemoryCloudSettings(DEFAULT_CLOUD_SETTINGS);
    return DEFAULT_CLOUD_SETTINGS as T;
  }
  if (command === "credential_env_var_present") return false as T;
  if (command === "read_engine_status")
    return {
      resolved: true,
      toolchain_identity: "8.1.2:blake3:qa-mock",
      ffmpeg_version: "8.1.2",
      ffmpeg_path: "/QA/ffmpeg",
      ffprobe_path: "/QA/ffprobe",
      capabilities: {
        has_zscale: true,
        has_h264_videotoolbox: true,
        has_prores_ks: true,
        has_lut3d: true,
        has_colortemperature: true,
      },
      error: null,
      note: "QA mock — not a real toolchain resolution",
    } as T;
  if (command === "native_media_capabilities") return { avFoundation: true, vision: true, caption: false, preview: false, audio: false, metal: false, osVersion: "qa", workerVersion: "qa", workerBlake3: "blake3:qa" } as T;
  if (command === "native_media_inspect_asset") return { duration: { numerator: 60, denominator: 1 }, videoTracks: [], audioTracks: [] } as T;
  if (command === "native_media_analyze_frames") return [] as T;
  if (command === "native_media_render_caption" || command === "native_media_render_preview") return { outputPath: "/QA/native.png", width: 1, height: 1, colorSpace: "sRGB", renderer: "qa" } as T;
  if (command === "native_media_audio_features") return { sampleRate: 48000, channelCount: 2, sampleCount: 0, rms: 0, peak: 0, zeroCrossingRate: 0, spectralFlux: 0, envelope: [], classification: null, classificationConfidence: null, classifierRevision: null } as T;
  if (command === "native_media_cancel") return undefined as T;
  if (command === "create_security_scoped_bookmark")
    return `qa-bookmark:${String(args.path)}` as T;
  if (command === "resolve_security_scoped_bookmark") {
    const bookmark = String(args.bookmark);
    if (!bookmark.startsWith("qa-bookmark:"))
      throw new Error("bookmark_resolve_failed_or_stale");
    return { token: 1, path: bookmark.slice("qa-bookmark:".length), stale: false, refreshedBookmark: null } as T;
  }
  throw new Error(`QA mock has no ${command}`);
}
export const tc = (value = 0) => {
  const s = Math.max(0, Math.floor(value / 1000));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
};
export const asset = (path?: string | null) =>
  path ? (qa ? path : convertFileSrc(path)) : undefined;

export type FinishVariant = {
  id: string;
  label?: string;
  preview_url?: string | null;
  source_hashes: string[];
  score?: number;
};

export async function readFinishVariants(interventionId: string): Promise<FinishVariant[]> {
  const result = await call<{ variants: FinishVariant[] }>("finish_read_variants", { intervention_id: interventionId });
  return result.variants.slice(0, 5);
}

export async function commitFinishVariant(variantId: string, lockedCutHash: string, sourceHashes: readonly string[] = []) {
  return call<{ variantId: string; lockedCutHash: string; sourceHashes: string[] }>("finish_commit_variant", {
    variant_id: variantId, locked_cut_hash: lockedCutHash, source_hashes: sourceHashes,
  });
}
