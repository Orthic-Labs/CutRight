// apps/studio/src/AssetsAuditions.test.tsx — CR-V2-B6-015.
import { describe, it, expect } from "vitest";
import { AssetPanel } from "./components/AssetPanel";
import { AuditionPanel } from "./components/AuditionPanel";
import { useAssets } from "./hooks/useAssets";
describe("AssetsAuditions", () => {
  it("AssetPanel renders", () => { expect(AssetPanel({ assets: [] })).toBeTruthy(); });
  it("AuditionPanel renders", () => { expect(AuditionPanel({ auditions: [] })).toBeTruthy(); });
  it("useAssets empty", () => { expect(useAssets("p1").selected).toEqual([]); });
});
