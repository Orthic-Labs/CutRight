// apps/studio/src/hooks/useAssets.ts — CR-V2-B6-015.
import { useState } from "react";
export function useAssets(project_id: string) {
  const [selected, setSelected] = useState<readonly string[]>([]);
  const [rejected, setRejected] = useState<readonly string[]>([]);
  return { project_id, selected, setSelected, rejected, setRejected };
}
