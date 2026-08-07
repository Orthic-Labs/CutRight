// apps/studio/src/hooks/useStudioRouter.ts — CR-V2-B6-022.
import { useState } from "react";
export type StudioModeV2 = "home"|"sources"|"transcript"|"story"|"beats"|"timeline"|"design"|"motion-sound"|"compare"|"finals"|"qa"|"settings";
export interface RouteState { project_id?: string; revision?: string; timeline_id?: string; mode: StudioModeV2; evidence_id?: string; object_id?: string }
export function useStudioRouter(initial: RouteState) {
  const [route, setRoute] = useState<RouteState>(initial);
  return { route, setRoute };
}
