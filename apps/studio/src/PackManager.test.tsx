import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import { PackManager } from "./modes/PackManagerMode";

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
  it("lists every pack with its actions", () => {
    render(<PackManager packs={packs} />);
    expect(screen.getByTestId("pack-manager")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /verify/i }).length).toBe(2);
  });

  it("rejects action on a non-local source", () => {
    const packsWithBad = [
      { ...packs[0], source: "remote" as const },
    ];
    render(<PackManager packs={packsWithBad} />);
    fireEvent.click(screen.getByTestId("verify-creator-minimal") as Element);
    // The element should not exist; the button uses data-action=verify.
    const btns = screen.getAllByRole("button", { name: /verify/i });
    fireEvent.click(btns[0]);
    expect(screen.getByRole("status").textContent).toMatch(
      /refuses to proceed/i
    );
  });

  it("activates a pack when its signature is valid", () => {
    render(<PackManager packs={packs} />);
    const actBtns = screen.getAllByRole("button", { name: /activate/i });
    fireEvent.click(actBtns[0]);
    expect(screen.getByRole("status").textContent).toMatch(/activate/i);
  });
});
