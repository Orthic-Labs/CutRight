import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { Help } from "./Help";

afterEach(() => {
  delete globalThis.__RIGHTKIT_QA_PLATFORM__;
  document.body.replaceChildren();
});

describe("Help", () => {
  it("renders platform-formatted modifier chords", async () => {
    globalThis.__RIGHTKIT_QA_PLATFORM__ = "windows";
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);

    await act(async () => root.render(<Help close={() => undefined} />));

    expect(host.textContent).toContain("Ctrl");
    expect(host.textContent).toContain("K");
    expect(host.textContent).toContain("Esc");
    await act(async () => root.unmount());
  });
});
