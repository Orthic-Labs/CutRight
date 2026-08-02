import { useCallback, useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { call, qa } from "../lib/api";
import { fixture, fixtureWords } from "../fixtures/qa";
import type { DecisionRecord, DecisionReplay, VariantSelection } from "../contracts/review";
import type { Word } from "../word-lock";
import type { Mode, Snapshot } from "../types";

// Owns the current project snapshot, the active mode/source/variant
// selection, transcript words, and the `load()` flow that opens a project
// and populates everything downstream. Moved out of main.tsx's `App()` per
// REV2 §14.4 `hooks/useProject.ts` — pure move, no behavior change.
//
// `load()` also needs to seed the review-ledger state (decisions,
// malformed lines, the current variant selection, and the final preset)
// that `useReviewLedger` owns; since that hook is composed after this one
// in `App.tsx`, its setters are threaded in as plain callback props exactly
// as the original single-component `App()` closure called them inline.
export function useProject(ledger: {
  setDecisions: (records: DecisionRecord[]) => void;
  setMalformedLines: (lines: DecisionReplay["malformed_lines"]) => void;
  setSelection: (selection: VariantSelection | null) => void;
  setFinalPreset: Dispatch<SetStateAction<string>>;
}) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(
    qa ? fixture : null,
  );
  const [mode, setMode] = useState<Mode>(qa ? "compare" : "sources");
  const [sourceIndex, setSourceIndex] = useState(0);
  const [variant, setVariant] = useState("natural");
  const [words, setWords] = useState<Record<string, Word[]>>(
    qa ? fixtureWords : {},
  );
  const [sourceWords, setSourceWords] = useState<Word[]>(
    qa ? fixtureWords.natural : [],
  );
  const [error, setError] = useState("");

  const selected = snapshot?.sources[sourceIndex];
  const available = {
    sources: Boolean(snapshot?.sources.length),
    compare: Boolean(
      snapshot?.variants.some((item) => item.mp4) &&
        snapshot?.variants.filter((item) => item.mp4).length === 2,
    ),
    finals: Boolean(snapshot?.finals.length),
    qa: Boolean(snapshot?.qa),
    // Settings is project config, not project content — it never depends
    // on what has been generated yet, so it's always reachable once a
    // project is open (same gate every other mode is already behind).
    settings: true,
  };

  const load = useCallback(async (path?: string) => {
    try {
      setError("");
      const picked = path ?? (await call<string | null>("pick_project"));
      if (!picked) return;
      const next = await call<Snapshot>("read_snapshot", { path: picked });
      setSnapshot(next);
      ledger.setFinalPreset((current) => current || next.finals[0]?.preset || "");
      localStorage.setItem("cutright-project", picked);
      const initial: Mode = next.finals.length
        ? "finals"
        : next.variants.length
          ? "compare"
          : "sources";
      setMode((localStorage.getItem("cutright-mode") as Mode) ?? initial);
      const history = await call<DecisionReplay>("read_decisions", {
        path: picked,
      });
      ledger.setDecisions(history.records);
      ledger.setMalformedLines(history.malformed_lines);
      const selectedVariant = await call<VariantSelection | null>(
        "read_variant_selection",
        { path: picked },
      );
      ledger.setSelection(selectedVariant);
      if (selectedVariant?.variant)
        ledger.setFinalPreset(() => selectedVariant.variant);
    } catch (reason) {
      setError(String(reason));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (snapshot && mode === "compare" && !words[variant])
      call<{ words: Word[] }>("read_transcript", {
        path: snapshot.project_path,
        variant,
      }).then((next) => setWords((old) => ({ ...old, [variant]: next.words })));
  }, [snapshot, mode, variant, words]);
  useEffect(() => {
    if (snapshot && mode === "sources" && selected?.transcript)
      call<{ words: Word[] }>("read_transcript", {
        path: snapshot.project_path,
        variant: selected.source_id,
      })
        .then((next) => setSourceWords(next.words))
        .catch(() => setSourceWords([]));
  }, [snapshot, mode, selected?.source_id, selected?.transcript]);
  useEffect(() => {
    localStorage.setItem("cutright-mode", mode);
  }, [mode]);

  return {
    snapshot,
    setSnapshot,
    mode,
    setMode,
    sourceIndex,
    setSourceIndex,
    variant,
    setVariant,
    words,
    setWords,
    sourceWords,
    setSourceWords,
    error,
    setError,
    selected,
    available,
    load,
  };
}
