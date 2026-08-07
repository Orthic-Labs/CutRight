// apps/studio/src/hooks/useTimeline.ts — CR-V2-B6-012.
import { useState } from "react";
export function useTimeline(initial_revision: string) {
  const [revision, setRevision] = useState(initial_revision);
  const [playhead_ms, setPlayhead] = useState(0);
  const [selection, setSelection] = useState<readonly string[]>([]);
  return { revision, setRevision, playhead_ms, setPlayhead, selection, setSelection };
}
