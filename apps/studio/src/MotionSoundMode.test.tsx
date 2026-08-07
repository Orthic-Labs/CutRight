// apps/studio/src/MotionSoundMode.test.tsx — CR-V2-B6-014.
import { describe, it, expect } from "vitest";
import { MotionSoundMode } from "./modes/MotionSoundMode";
import { TransientMarker } from "./components/audio";
describe("MotionSoundMode", () => {
  it("renders", () => { expect(MotionSoundMode({ timeline_id: "tl1" })).toBeTruthy(); });
  it("TransientMarker renders", () => { expect(TransientMarker({ transient_id: "t1", position_ms: 100 })).toBeTruthy(); });
});
