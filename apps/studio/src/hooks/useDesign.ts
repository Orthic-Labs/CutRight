// apps/studio/src/hooks/useDesign.ts — CR-V2-B6-013.
import { useState } from "react";
export function useDesign(revision_id: string) {
  const [accepted_direction, setDirection] = useState<string | null>(null);
  return { revision_id, accepted_direction, setDirection };
}
