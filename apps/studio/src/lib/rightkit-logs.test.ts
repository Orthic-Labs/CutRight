import { describe, expect, it, vi } from "vitest";
import {
  createStudioLogs,
  type StudioDiagnosticsBundle,
} from "./rightkit-logs";
import type { LogsInvoke } from "@rightkit/logs";

const bundle: StudioDiagnosticsBundle = {
  schema_version: 1,
  metadata: {
    app_name: "cutright",
    app_version: "qa",
    build: "qa",
    os: "qa",
    settings: { offline: true, telemetry: false },
  },
  sources: [],
};

describe("createStudioLogs", () => {
  it("writes only content-free fields to the project-scoped RightKit command", async () => {
    const mockInvoke = vi.fn<
      (command: string, args?: Record<string, unknown>) => Promise<unknown>
    >();
    mockInvoke.mockResolvedValue(undefined);
    const invoke: LogsInvoke = <T>(
      command: string,
      args?: Record<string, unknown>,
    ) => mockInvoke(command, args) as Promise<T>;
    const logs = createStudioLogs(() => "/projects/demo", invoke);

    logs.info("project.opened", {
      operation: "open",
      source_count: 2,
      transcript: "private words",
    });
    await logs.flush();

    expect(mockInvoke).toHaveBeenCalledWith("rightkit_logs_write", {
      path: "/projects/demo",
      events: [expect.objectContaining({
        schema_version: 1,
        event: "project.opened",
        operation: "open",
        source_count: 2,
      })],
    });
    const args = mockInvoke.mock.calls[0]?.[1] as unknown as {
      events: Array<Record<string, unknown>>;
    };
    expect(args.events[0]).not.toHaveProperty("transcript");
  });

  it("returns the typed native diagnostics bundle", async () => {
    const mockInvoke = vi.fn<
      (command: string, args?: Record<string, unknown>) => Promise<unknown>
    >();
    mockInvoke.mockImplementation(async (command) =>
      command === "rightkit_logs_collect" ? bundle : undefined,
    );
    const invoke: LogsInvoke = <T>(
      command: string,
      args?: Record<string, unknown>,
    ) => mockInvoke(command, args) as Promise<T>;
    const logs = createStudioLogs(() => "/projects/demo", invoke);

    await expect(logs.collectDiagnostics<StudioDiagnosticsBundle>()).resolves.toEqual(bundle);
    expect(mockInvoke).toHaveBeenCalledWith("rightkit_logs_collect", { path: "/projects/demo" });
  });
});
