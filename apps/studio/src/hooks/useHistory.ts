// apps/studio/src/hooks/useHistory.ts — CR-V2-B6-016.
import { useState } from "react";
export function useHistory() {
  const [undo_depth, setUndo] = useState(0);
  const [redo_depth, setRedo] = useState(0);
  return { undo_depth, redo_depth, setUndo, setRedo, can_undo: undo_depth > 0, can_redo: redo_depth > 0 };
}
