// apps/studio/src/StoryBeats.test.tsx — CR-V2-B6-009.
import { describe, it, expect } from "vitest";
import { StoryMode } from "./modes/StoryMode";
import { TakeComparison } from "./components/TakeComparison";
describe("StoryBeats", () => {
  it("StoryMode renders beats", () => { expect(StoryMode({ beats: [] })).toBeTruthy(); });
  it("TakeComparison renders selected", () => {
    expect(TakeComparison({ selected: { take_id: "t1", signals: [] }, alternates: [], confidence: 0.5 })).toBeTruthy();
  });
});
