import { useState } from "react";
import { call, qa } from "../lib/api";
import { memoryDecisions, memoryMalformed } from "../fixtures/qa";
import {
  buildSegmentFlagIntent,
  SCHEMA_VERSION,
  SOURCE_WORD_ID,
  WORD_ID,
  type DecisionIntent,
  type DecisionReason,
  type DecisionRecord,
  type DecisionReplay,
  type RelinkResult,
  type ReviewTarget,
  type SourceCheck,
  type VariantSelection,
} from "../contracts/review";
import type { Word } from "../word-lock";
import type { Mode, Snapshot, Variant } from "../types";

// Owns the decision ledger (approve/reject/flag verdicts, the replay
// history, malformed-line reporting), source verification/relinking, and
// the reviewed-base final selection. Moved out of main.tsx's `App()` per
// REV2 §14.4 `hooks/useReviewLedger.ts` — pure move, no behavior change.
export function useReviewLedger({
  snapshot,
  mode,
  variant,
  cursor,
  playheadRef,
  activeVariant,
  setError,
}: {
  snapshot: Snapshot | null;
  mode: Mode;
  variant: string;
  cursor: { word?: Word };
  // A ref, not the committed `playhead` state: usePlayback only commits
  // that state on a cursor-word change or a whole-second tick, so it can
  // lag several hundred ms behind the real position during continuous
  // playback. The ref is always current, which is what decision-provenance
  // timestamps (`playhead_ms` below) need.
  playheadRef: { current: number };
  activeVariant: Variant | undefined;
  setError: (message: string) => void;
}) {
  const [reasonKind, setReasonKind] = useState<"approved" | "rejected" | null>(
    null,
  );
  const [note, setNote] = useState("");
  // QA mode boots straight onto the fixture project, so its ledger starts
  // with the same replay the native load() path fetches.
  const [decisions, setDecisions] = useState<DecisionRecord[]>(
    qa ? [...memoryDecisions] : [],
  );
  const [malformedLines, setMalformedLines] = useState<
    DecisionReplay["malformed_lines"]
  >(qa ? [...memoryMalformed] : []);
  const [sessionCount, setSessionCount] = useState(0);
  const [ledgerOpen, setLedgerOpen] = useState(false);
  const [checks, setChecks] = useState<SourceCheck[] | null>(null);
  const [verifying, setVerifying] = useState(false);
  const [verifyProgress, setVerifyProgress] = useState("");
  const [relinks, setRelinks] = useState<Record<string, RelinkResult>>({});
  const [selection, setSelection] = useState<VariantSelection | null>(null);
  const [selecting, setSelecting] = useState<string | null>(null);
  const [flagging, setFlagging] = useState(false);
  const [lastFlag, setLastFlag] = useState<{
    segmentId: string;
    reason: DecisionReason;
  } | null>(null);
  const [finalPreset, setFinalPreset] = useState(
    qa ? snapshot?.finals[0]?.preset ?? "" : "",
  );

  async function commit(reason: DecisionReason) {
    if (!snapshot || !reasonKind) return;
    // Sources and QA modes never produce a variant/final verdict.
    if (mode !== "compare" && mode !== "finals") return;
    const target: ReviewTarget =
      mode === "finals"
        ? { target_kind: "final", preset: finalPreset }
        : { target_kind: "variant", variant };
    const word = cursor.word;
    const intent: DecisionIntent = {
      schema_version: SCHEMA_VERSION,
      client_request_id: crypto.randomUUID(),
      target,
      verdict: reasonKind,
      reason,
      note: reason === "other" ? note.trim() : null,
      playhead_ms: Math.round(playheadRef.current),
      word_id: word?.id && WORD_ID.test(word.id) ? word.id : null,
      source_word_id:
        word?.source_word_id && SOURCE_WORD_ID.test(word.source_word_id)
          ? word.source_word_id
          : null,
    };
    try {
      const record = await call<DecisionRecord>("append_decision", {
        path: snapshot.project_path,
        intent,
      });
      setDecisions((old) => [...old, record]);
      setSessionCount((count) => count + 1);
      setReasonKind(null);
      setNote("");
      setError("");
    } catch (cause) {
      setError(String(cause));
    }
  }
  async function acknowledgeQa() {
    if (!snapshot) return;
    const intent: DecisionIntent = {
      schema_version: SCHEMA_VERSION,
      client_request_id: crypto.randomUUID(),
      target: { target_kind: "qa_report", preset: null },
      verdict: "acknowledged",
      reason: "reviewed",
      note: null,
      playhead_ms: 0,
      word_id: null,
      source_word_id: null,
    };
    try {
      const record = await call<DecisionRecord>("append_decision", {
        path: snapshot.project_path,
        intent,
      });
      setDecisions((old) => [...old, record]);
      setSessionCount((count) => count + 1);
      setError("");
    } catch (cause) {
      setError(String(cause));
    }
  }
  async function verifySources() {
    if (!snapshot) return;
    setVerifying(true);
    setChecks(null);
    setVerifyProgress("");
    let unlisten: (() => void) | undefined;
    try {
      if (!qa) {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<{ completed: number; total: number }>(
          "source-verify-progress",
          (event) =>
            setVerifyProgress(`${event.payload.completed}/${event.payload.total}`),
        );
      }
      const results = await call<SourceCheck[]>("verify_sources", {
        path: snapshot.project_path,
      });
      setChecks(results);
      setError("");
    } catch (cause) {
      setError(String(cause));
    } finally {
      unlisten?.();
      setVerifying(false);
      setVerifyProgress("");
    }
  }
  async function relink(sourceId: string) {
    if (!snapshot) return;
    const current =
      checks?.find((check) => check.source_id === sourceId)?.path ?? "";
    // QA mode simulates the file pick; the native build asks for the path.
    const picked = qa
      ? `/QA/media/${sourceId}-relinked.mp4`
      : window.prompt(`New path for ${sourceId}`, current);
    if (!picked) return;
    try {
      const result = await call<RelinkResult>("relink_source", {
        path: snapshot.project_path,
        source_id: sourceId,
        new_path: picked,
      });
      setRelinks((old) => ({ ...old, [sourceId]: result }));
      setChecks((old) =>
        old?.map((check) =>
          check.source_id === sourceId
            ? {
                ...check,
                path: result.path,
                actual_blake3: result.blake3,
                matches: result.matches,
                error: null,
              }
            : check,
        ) ?? old,
      );
      setError("");
    } catch (cause) {
      setError(String(cause));
    }
  }
  async function useFinal(preset: string) {
    if (!snapshot) return;
    setSelecting(preset);
    try {
      const next = await call<VariantSelection>("select_variant", {
        path: snapshot.project_path,
        variant: preset,
      });
      setSelection(next);
      setFinalPreset(preset);
      setError("");
    } catch (cause) {
      setError(String(cause));
    } finally {
      setSelecting(null);
    }
  }
  function segmentAtPlayhead(): string | null {
    const segments = activeVariant?.cut_plan?.segments ?? [];
    if (!segments.length) return null;
    const at = playheadRef.current;
    const idAt = (index: number) =>
      segments[index].id ?? `segment-${String(index + 1).padStart(3, "0")}`;
    const containing = segments.findIndex(
      (segment) =>
        at >= (segment.output_start_ms ?? 0) &&
        at < (segment.output_end_ms ?? Infinity),
    );
    if (containing >= 0) return idAt(containing);
    let nearest = 0;
    let distance = Infinity;
    segments.forEach((segment, index) => {
      const d = Math.min(
        Math.abs(at - (segment.output_start_ms ?? 0)),
        Math.abs(at - (segment.output_end_ms ?? 0)),
      );
      if (d < distance) {
        distance = d;
        nearest = index;
      }
    });
    return idAt(nearest);
  }
  async function commitSegment(reason: DecisionReason) {
    if (!snapshot) return;
    const segmentId = segmentAtPlayhead();
    if (!segmentId) {
      setError("No segments in the active cut plan to flag");
      return;
    }
    const word = cursor.word;
    try {
      const intent = buildSegmentFlagIntent({
        variant,
        segment_id: segmentId,
        reason,
        note: reason === "other" ? note.trim() : null,
        playhead_ms: Math.round(playheadRef.current),
        word_id: word?.id && WORD_ID.test(word.id) ? word.id : null,
        source_word_id:
          word?.source_word_id && SOURCE_WORD_ID.test(word.source_word_id)
            ? word.source_word_id
            : null,
      });
      const record = await call<DecisionRecord>("append_decision", {
        path: snapshot.project_path,
        intent,
      });
      setDecisions((old) => [...old, record]);
      setSessionCount((count) => count + 1);
      setLastFlag({ segmentId, reason });
      setFlagging(false);
      setNote("");
      setError("");
    } catch (cause) {
      setError(String(cause));
    }
  }

  return {
    reasonKind,
    setReasonKind,
    note,
    setNote,
    decisions,
    setDecisions,
    malformedLines,
    setMalformedLines,
    sessionCount,
    ledgerOpen,
    setLedgerOpen,
    checks,
    verifying,
    verifyProgress,
    relinks,
    selection,
    setSelection,
    selecting,
    flagging,
    setFlagging,
    lastFlag,
    finalPreset,
    setFinalPreset,
    commit,
    acknowledgeQa,
    verifySources,
    relink,
    useFinal,
    commitSegment,
  };
}
