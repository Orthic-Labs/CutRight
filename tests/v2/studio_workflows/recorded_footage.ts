// tests/v2/studio_workflows/recorded_footage.ts — CR-V2-B6-025.
export const LANES = ["recorded_footage","repurpose","explainer","anchored_creative"] as const;
export type Lane = (typeof LANES)[number];
