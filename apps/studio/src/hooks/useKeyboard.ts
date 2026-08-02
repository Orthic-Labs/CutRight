import { useEffect, type Dispatch, type SetStateAction } from "react";
import type { Mode } from "../types";

// The global keydown handler: mode switches, transport controls,
// word/segment stepping, approve/reject/flag shortcuts, and the command
// palette/help/ledger toggles. Moved out of main.tsx's `App()` per REV2
// §14.4 `hooks/useKeyboard.ts` — pure move, no behavior change, including
// the original effect's lack of a dependency array (it re-subscribes on
// every render so the closure always sees current state).
export function useKeyboard({
  active,
  mode,
  setMode,
  available,
  setPalette,
  setHelp,
  setReasonKind,
  setFlagging,
  setLedgerOpen,
  setSourceIndex,
  sourceCount,
  reload,
  playhead,
  seek,
  togglePlayback,
  pause,
  playOrIncreaseRate,
  swap,
  seekWord,
  seekSegment,
  frameDuration,
}: {
  active: boolean;
  mode: Mode;
  setMode: (mode: Mode) => void;
  available: Record<Mode, boolean>;
  setPalette: (value: boolean) => void;
  setHelp: (value: boolean) => void;
  setReasonKind: (value: "approved" | "rejected" | null) => void;
  setFlagging: (value: boolean) => void;
  setLedgerOpen: Dispatch<SetStateAction<boolean>>;
  setSourceIndex: (value: number) => void;
  sourceCount: number;
  reload: () => void;
  playhead: number;
  seek: (ms: number) => void;
  togglePlayback: () => void;
  pause: () => void;
  playOrIncreaseRate: () => void;
  swap: () => void;
  seekWord: (direction: number) => void;
  seekSegment: (direction: number) => void;
  frameDuration: () => number;
}) {
  useEffect(() => {
    if (!active) return;
    const onKey = (event: KeyboardEvent) => {
      if (
        event.target instanceof HTMLInputElement ||
        event.target instanceof HTMLTextAreaElement
      ) {
        if (event.key === "Escape") {
          setReasonKind(null);
          setFlagging(false);
        }
        return;
      }
      if (
        (event.metaKey || event.ctrlKey) &&
        ["1", "2", "3", "4"].includes(event.key)
      ) {
        event.preventDefault();
        const next = ["sources", "compare", "finals", "qa"][
          Number(event.key) - 1
        ] as Mode;
        if (available[next]) setMode(next);
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPalette(true);
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "r") {
        event.preventDefault();
        reload();
        return;
      }
      if (event.key === "?" && !event.metaKey) setHelp(true);
      if (event.key === "Escape") {
        setHelp(false);
        setPalette(false);
        setReasonKind(null);
        setFlagging(false);
        setLedgerOpen(false);
      }
      if (event.key === " " && mode !== "qa") {
        event.preventDefault();
        togglePlayback();
      }
      if (event.key.toLowerCase() === "j") seek(playhead - 2000);
      if (event.key.toLowerCase() === "k") pause();
      if (event.key.toLowerCase() === "l") playOrIncreaseRate();
      if (event.key.toLowerCase() === "a" && mode === "compare") swap();
      if (
        event.key.toLowerCase() === "y" &&
        (mode === "compare" || mode === "finals")
      )
        setReasonKind("approved");
      if (
        event.key.toLowerCase() === "x" &&
        (mode === "compare" || mode === "finals")
      )
        setReasonKind("rejected");
      if (/^[1-9]$/.test(event.key) && !event.metaKey)
        setSourceIndex(Math.min(Number(event.key) - 1, sourceCount - 1));
      if (event.key === "ArrowRight")
        event.shiftKey ? seekSegment(1) : seekWord(1);
      if (event.key === "ArrowLeft")
        event.shiftKey ? seekSegment(-1) : seekWord(-1);
      if (event.key === ",") seek(playhead - frameDuration());
      if (event.key === ".") seek(playhead + frameDuration());
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  });
}
