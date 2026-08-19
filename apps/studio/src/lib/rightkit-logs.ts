import { createLogsClient, type LogsInvoke } from "@rightkit/logs";
import { call } from "./api";

export const STUDIO_LOG_FIELDS = [
  "dropped_count",
  "duration_ms",
  "error_code",
  "operation",
  "outcome",
  "source_count",
  "variant_count",
] as const;

export type StudioDiagnosticsBundle = {
  schema_version: 1;
  metadata: {
    app_name: string;
    app_version: string;
    build: string;
    os: string;
    settings: Record<string, unknown>;
  };
  sources: Array<{
    name: string;
    file_name: string;
    events: Array<Record<string, unknown>>;
    stats: {
      scanned_bytes: number;
      parsed: number;
      malformed: number;
      dropped: number;
      error_anchor_found: boolean;
      rotated_read: boolean;
    };
    warnings: string[];
  }>;
};

/** Studio preserves its project-scoped native log store behind RightKit's IPC contract. */
export function createStudioLogs(
  projectPath: () => string | null,
  invoke: LogsInvoke = call,
) {
  return createLogsClient({
    invoke: (command, args) => {
      const path = projectPath();
      if (!path) return Promise.reject(new Error("diagnostics require an open project"));
      return invoke(command, { ...args, path });
    },
    allowedFields: STUDIO_LOG_FIELDS,
  });
}
