import { describe, expect, it } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { PackManager } from "./modes/PackManagerMode";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const packs = [
  {
    id: "creator-minimal",
    active: true,
    compatible: ["creator-2"],
    signature_valid: true,
    size: 1024,
    source: "local_verified_bundle" as const,
    available: true,
    measured_target_status: "supported" as const,
  },
  {
    id: "creator-2",
    active: false,
    compatible: ["creator-minimal"],
    signature_valid: false,
    size: 2048,
    source: "local_verified_bundle" as const,
    available: true,
    measured_target_status: "supported" as const,
  },
];

describe("PackManager", () => {
  it("lists every pack with its actions", async () => {
    const host = await renderPackManager(packs);
    expect(host.querySelector("[data-testid='pack-manager']")).toBeTruthy();
    expect(host.querySelectorAll("[data-action='verify']")).toHaveLength(2);
  });

  it("refuses an action on a non-local source", async () => {
    const packsWithBad = [
      { ...packs[0], source: "remote" as const },
    ];
    const host = await renderPackManager(packsWithBad);
    const verify = host.querySelector("[data-action='verify']")!;
    await act(async () => verify.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(host.querySelector("[role='status']")?.textContent).toMatch(/refuses to proceed/i);
  });

  it("activates a pack when its signature is valid", async () => {
    const host = await renderPackManager(packs);
    const activate = host.querySelector("[data-pack-id='creator-minimal'] [data-action='activate']")!;
    await act(async () => activate.dispatchEvent(new MouseEvent("click", { bubbles: true })));
    expect(host.querySelector("[role='status']")?.textContent).toMatch(/activate creator-minimal/i);
  });
});

async function renderPackManager(packs: Parameters<typeof PackManager>[0]["packs"]) {
  const host = document.createElement("div");
  document.body.append(host);
  await act(async () => createRoot(host).render(<PackManager packs={packs} />));
  return host;
}
