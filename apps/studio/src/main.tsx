import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createRoot } from "react-dom/client";
import { convertFileSrc, invoke as tauriInvoke } from "@tauri-apps/api/core";
import { findWord, swapTarget, type Word } from "./word-lock";
import { cutMarkers, type CutMarker } from "./cut-markers";
import {
  buildMockRecord,
  buildSegmentFlagIntent,
  REASONS,
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
} from "./contracts/review";
import "./styles.css";

type Mode = "sources" | "compare" | "finals" | "qa";

// The true state of an optional filesystem artifact (REV2 §12.1): a `Ready`
// artifact is distinct from `Missing` (never generated), which is distinct
// from `Invalid` (generated but failed to parse — corruption, not absence)
// and `Stale` (parsed fine but superseded by a newer render/edit). The
// backend still sends the original `qa`/`bench`/`cut_plan` fields for
// compatibility; these `*_artifact` fields carry the corrected state.
type ArtifactState<T> =
  | { state: "missing" }
  | { state: "ready"; data: T }
  | { state: "invalid"; data: { path: string; error: string } }
  | { state: "stale"; data: { path: string; reason: string } };

function artifactIssue<T>(state?: ArtifactState<T> | null): string | null {
  if (!state) return null;
  if (state.state === "invalid") return `corrupt — ${state.data.error}`;
  if (state.state === "stale") return `stale — ${state.data.reason}`;
  return null;
}

type Variant = {
  id: string;
  mp4?: string | null;
  fps?: number;
  duration_ms?: number;
  cut_plan?: { segments?: Segment[] } | null;
  cut_plan_artifact?: ArtifactState<{ segments?: Segment[] }> | null;
};
type Segment = {
  id?: string;
  source_id?: string;
  source_start_ms?: number;
  source_end_ms?: number;
  output_start_ms?: number;
  output_end_ms?: number;
  label?: string;
};
// Outcome of the backend's exact-file asset-scope grant for one registered
// source (REV2 §12.4): `granted` means playback scope was extended to this
// file; `verified` means its current BLAKE3 matches the manifest. A source
// can be granted-but-unverified (still playable, flagged) or ungranted
// entirely (not a regular file, or fails to probe as supported media).
type SourceIntegrity = {
  source_id: string;
  path: string;
  granted: boolean;
  verified: boolean;
  error?: string | null;
} | null;
type Source = {
  source_id: string;
  path?: string;
  display_name?: string;
  duration_ms?: number;
  width?: number;
  height?: number;
  is_hdr?: boolean;
  file_present?: boolean;
  stages?: Record<string, boolean>;
  transcript?: string | null;
  poster_jpg?: string | null;
  waveform_png?: string | null;
  integrity?: SourceIntegrity;
};
type Snapshot = {
  project_path: string;
  generated_at: string;
  project_revision?: string;
  project_instance_id?: string;
  manifest: { project_id?: string; title?: string };
  sources: Source[];
  stages: Record<string, boolean>;
  variants: Variant[];
  finals: Array<{
    preset: string;
    aspect?: string;
    mp4?: string | null;
    duration_ms?: number;
    width?: number;
    height?: number;
  }>;
  qa?: {
    status?: "pass" | "fail";
    checks?: Array<{ id: string; status: "pass" | "fail"; evidence?: string }>;
  } | null;
  qa_artifact?: ArtifactState<Snapshot["qa"]> | null;
  bench?: { decision?: string };
  bench_artifact?: ArtifactState<{ decision?: string; report?: string }> | null;
  decisions_path?: string;
};
const qa =
  new URLSearchParams(location.search).has("qa") ||
  import.meta.env.VITE_CUTRIGHT_QA === "1";
const fixtureWords: Record<string, Word[]> = {
  natural: [
    "This",
    "is",
    "the",
    "same",
    "spoken",
    "sentence",
    "with",
    "room",
    "to",
    "breathe",
  ].map((text, i) => ({
    id: `ow_${String(i).padStart(6, "0")}`,
    text,
    start_ms: i * 620,
    end_ms: i * 620 + 510,
    source_word_id: `source-a:w_${String(i).padStart(6, "0")}`,
  })),
  tight: ["This", "is", "the", "same", "spoken", "sentence", "breathe"].map(
    (text, i) => ({
      id: `ow_${String(i).padStart(6, "0")}`,
      text,
      start_ms: i * 490,
      end_ms: i * 490 + 410,
      source_word_id: `source-a:w_${String(i < 6 ? i : 9).padStart(6, "0")}`,
    }),
  ),
};
const fixture: Snapshot = {
  project_path: "/QA/Studio.video-project",
  generated_at: "2026-07-26T09:14:22Z",
  manifest: { project_id: "qa-demo", title: "Night walk" },
  sources: [
    {
      source_id: "source-a",
      display_name: "Clip 01 — street",
      duration_ms: 252000,
      width: 3840,
      height: 2160,
      is_hdr: true,
      file_present: true,
      transcript: "analysis/transcripts/source-a.json",
      stages: {
        ingested: true,
        transcribed: true,
        analyzed: true,
        in_candidates: true,
        in_cut: true,
      },
    },
    {
      source_id: "source-b",
      display_name: "Clip 02 — door",
      duration_ms: 104000,
      width: 1920,
      height: 1080,
      file_present: true,
      transcript: "analysis/transcripts/source-b.json",
      stages: {
        ingested: true,
        transcribed: true,
        analyzed: true,
        in_candidates: true,
        in_cut: false,
      },
    },
  ],
  stages: {
    ingested: true,
    transcribed: true,
    analyzed: true,
    candidates: true,
    rough_cut: true,
    final_render: true,
    qa: true,
  },
  variants: [
    {
      id: "natural",
      mp4: "/QA/Studio.video-project/render/rough-cuts/natural.mp4",
      duration_ms: 6200,
      fps: 29.97,
      cut_plan: {
        segments: [
          {
            id: "segment-001",
            output_start_ms: 0,
            output_end_ms: 3100,
            source_start_ms: 0,
            source_end_ms: 3100,
          },
          {
            id: "segment-002",
            output_start_ms: 3100,
            output_end_ms: 6200,
            source_start_ms: 4400,
            source_end_ms: 7500,
          },
        ],
      },
    },
    {
      id: "tight",
      mp4: "/QA/Studio.video-project/render/rough-cuts/tight.mp4",
      duration_ms: 4000,
      fps: 29.97,
      cut_plan: {
        segments: [
          { id: "segment-001", output_start_ms: 0, output_end_ms: 2000 },
          { id: "segment-002", output_start_ms: 2000, output_end_ms: 4000 },
        ],
      },
    },
  ],
  finals: [
    {
      preset: "youtube",
      aspect: "16:9",
      duration_ms: 6200,
      width: 1920,
      height: 1080,
    },
    {
      preset: "reels",
      aspect: "9:16",
      duration_ms: 6100,
      width: 1080,
      height: 1920,
    },
  ],
  qa: {
    status: "pass",
    checks: [
      { id: "Container", status: "pass" },
      { id: "Captions", status: "pass" },
      { id: "Duration", status: "pass" },
    ],
  },
  bench: { decision: "unresolved" },
  decisions_path: "/QA/Studio.video-project/feedback/decisions.jsonl",
};

// Seeded history so browser QA exercises the replay states without the native
// backend: one stale artifact record plus one malformed ledger line.
const seedStale: DecisionRecord = {
  ...buildMockRecord(
    {
      schema_version: SCHEMA_VERSION,
      client_request_id: "qa-seed-stale",
      target: { target_kind: "variant", variant: "tight" },
      verdict: "rejected",
      reason: "pacing",
      note: null,
      playhead_ms: 2400,
      word_id: "ow_000004",
      source_word_id: "source-a:w_000004",
    },
    fixture.manifest.project_id ?? "qa-demo",
    0,
  ),
  decision_id: "d_mock_seed",
  ts: "2026-07-25T18:03:11Z",
  status: "stale_artifact",
};
const memoryDecisions: DecisionRecord[] = [seedStale];
const memoryMalformed: DecisionReplay["malformed_lines"] = [
  {
    line_number: 4,
    content: '{"decision_id":"d_trunc',
    error: "EOF while parsing a value",
  },
];
let memorySelection: VariantSelection | null = null;
async function call<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (!qa) return tauriInvoke<T>(command, args);
  if (command === "pick_project") return fixture.project_path as T;
  if (command === "read_transcript")
    return {
      words: fixtureWords[String(args.variant)] ?? fixtureWords.natural,
    } as T;
  if (command === "read_decisions")
    return {
      records: memoryDecisions,
      malformed_lines: memoryMalformed,
    } as T;
  if (command === "append_decision") {
    const intent = args.intent as DecisionIntent;
    const existing = memoryDecisions.find(
      (record) => record.client_request_id === intent.client_request_id,
    );
    if (existing) return existing as T;
    const record = buildMockRecord(
      intent,
      fixture.manifest.project_id ?? "qa-demo",
      memoryDecisions.length + 1,
    );
    memoryDecisions.push(record);
    return record as T;
  }
  if (command === "verify_sources")
    // source-a verifies clean; source-b drifted so QA can exercise the
    // mismatch banner and the relink flow.
    return fixture.sources.map((source, index) => ({
      source_id: source.source_id,
      path: `/QA/media/${source.source_id}.mp4`,
      expected_blake3: `blake3:expected-${source.source_id}`,
      actual_blake3:
        index === 1
          ? "blake3:drifted-bytes"
          : `blake3:expected-${source.source_id}`,
      matches: index === 0,
      error: null,
    })) as T;
  if (command === "relink_source") {
    const sourceId = String(args.source_id);
    return {
      source_id: sourceId,
      path: String(args.new_path),
      blake3: `blake3:expected-${sourceId}`,
      matches: true,
    } as T;
  }
  if (command === "select_variant") {
    const variant = String(args.variant);
    memorySelection = {
      schema_version: 1,
      variant,
      rough_cut_path: `render/rough-cuts/${variant}.mp4`,
      rough_cut_blake3: `blake3:mock-${variant}-roughcut`,
      rough_cut_size: 12345678,
      selected_at: new Date().toISOString(),
      selected_by: "qa-mock",
    };
    return memorySelection as T;
  }
  if (command === "read_variant_selection") return memorySelection as T;
  throw new Error(`QA mock has no ${command}`);
}
const tc = (value = 0) => {
  const s = Math.max(0, Math.floor(value / 1000));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
};
const asset = (path?: string | null) =>
  path ? (qa ? path : convertFileSrc(path)) : undefined;

function App() {
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
  const [playhead, setPlayhead] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [error, setError] = useState("");
  const [help, setHelp] = useState(false);
  const [palette, setPalette] = useState(false);
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
    qa ? fixture.finals[0]?.preset ?? "" : "",
  );
  const [theme, setTheme] = useState(
    () =>
      localStorage.getItem("cutright-theme") ??
      (matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark"),
  );
  const videoRefs = useRef<Record<string, HTMLVideoElement | null>>({});
  const selected = snapshot?.sources[sourceIndex];
  const activeVariant = snapshot?.variants.find((item) => item.id === variant);
  const available = {
    sources: Boolean(snapshot?.sources.length),
    compare: Boolean(
      snapshot?.variants.some((item) => item.mp4) &&
        snapshot?.variants.filter((item) => item.mp4).length === 2,
    ),
    finals: Boolean(snapshot?.finals.length),
    qa: Boolean(snapshot?.qa),
  };
  const cursor = findWord(words[variant] ?? [], playhead);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("cutright-theme", theme);
  }, [theme]);
  const load = useCallback(async (path?: string) => {
    try {
      setError("");
      const picked = path ?? (await call<string | null>("pick_project"));
      if (!picked) return;
      const next = await call<Snapshot>("read_snapshot", { path: picked });
      setSnapshot(next);
      setFinalPreset((current) => current || next.finals[0]?.preset || "");
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
      setDecisions(history.records);
      setMalformedLines(history.malformed_lines);
      const selected = await call<VariantSelection | null>(
        "read_variant_selection",
        { path: picked },
      );
      setSelection(selected);
      if (selected?.variant) setFinalPreset(selected.variant);
    } catch (reason) {
      setError(String(reason));
    }
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
    if (!snapshot) return;
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
        void load(snapshot.project_path);
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
        setSourceIndex(
          Math.min(Number(event.key) - 1, snapshot.sources.length - 1),
        );
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
  useEffect(() => {
    localStorage.setItem("cutright-mode", mode);
  }, [mode]);
  useEffect(() => {
    let id = 0;
    const tick = () => {
      const video = videoRefs.current[mode === "compare" ? variant : "source"];
      if (video && !video.paused) setPlayhead(video.currentTime * 1000);
      id = requestAnimationFrame(tick);
    };
    if (playing) id = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(id);
  }, [playing, variant, mode]);
  function seek(ms: number, key = mode === "compare" ? variant : "source") {
    setPlayhead(Math.max(0, ms));
    const video = videoRefs.current[key];
    if (video) video.currentTime = Math.max(0, ms / 1000);
  }
  function activeVideo() {
    return videoRefs.current[mode === "compare" ? variant : "source"];
  }
  function togglePlayback() {
    const video = activeVideo();
    if (!video) return;
    if (video.paused) video.play().catch(() => setPlaying(false));
    else video.pause();
  }
  function pause() {
    activeVideo()?.pause();
    setPlaying(false);
  }
  function playOrIncreaseRate() {
    const video = activeVideo();
    if (!video) return;
    if (video.paused) {
      video.playbackRate = 1;
      video.play().catch(() => setPlaying(false));
      return;
    }
    video.playbackRate = video.playbackRate >= 4 ? 1 : video.playbackRate * 2;
  }
  function frameDuration() {
    return 1000 / Math.max(1, activeVariant?.fps ?? 30);
  }
  function seekWord(direction: number) {
    const list = words[variant] ?? [];
    const current = cursor.word ? list.indexOf(cursor.word) : -1;
    const target =
      list[Math.max(0, Math.min(list.length - 1, current + direction))];
    if (target) seek(target.start_ms);
  }
  function seekSegment(direction: number) {
    const segments = activeVariant?.cut_plan?.segments ?? [];
    const current = segments.findIndex(
      (segment) =>
        playhead >= (segment.output_start_ms ?? 0) &&
        playhead < (segment.output_end_ms ?? Infinity),
    );
    const index = Math.max(0, Math.min(segments.length - 1, current + direction));
    const target = segments[index];
    if (target) seek(target.output_start_ms ?? 0);
  }
  function swap() {
    const other = variant === "natural" ? "tight" : "natural";
    const target = swapTarget(
      words[variant] ?? [],
      words[other] ?? [],
      playhead,
    );
    if (target.refused) {
      setError(`${other} has no content`);
      return;
    }
    const outgoing = videoRefs.current[variant];
    const incoming = videoRefs.current[other];
    outgoing?.pause();
    setPlayhead(target.seek_ms);
    setVariant(other);
    if (incoming) {
      incoming.currentTime = target.seek_ms / 1000;
      const resume = () => {
        if (playing && !target.paused)
          incoming.play().catch(() => setPlaying(false));
        incoming.removeEventListener("seeked", resume);
      };
      incoming.addEventListener("seeked", resume);
      if (incoming.readyState >= 2) resume();
    }
    if (target.paused) setPlaying(false);
  }
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
      playhead_ms: Math.round(playhead),
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
    const idAt = (index: number) =>
      segments[index].id ?? `segment-${String(index + 1).padStart(3, "0")}`;
    const containing = segments.findIndex(
      (segment) =>
        playhead >= (segment.output_start_ms ?? 0) &&
        playhead < (segment.output_end_ms ?? Infinity),
    );
    if (containing >= 0) return idAt(containing);
    let nearest = 0;
    let distance = Infinity;
    segments.forEach((segment, index) => {
      const d = Math.min(
        Math.abs(playhead - (segment.output_start_ms ?? 0)),
        Math.abs(playhead - (segment.output_end_ms ?? 0)),
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
        playhead_ms: Math.round(playhead),
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
  if (!snapshot) return <Empty onOpen={() => load()} error={error} />;
  const latest = [...decisions]
    .reverse()
    .find(
      (decision) =>
        (mode === "compare" &&
          decision.kind === "variant_verdict" &&
          decision.variant === variant) ||
        (mode === "finals" &&
          decision.kind === "final_verdict" &&
          decision.preset === finalPreset),
    );
  const qaAcknowledged = decisions.some(
    (decision) => decision.kind === "qa_ack",
  );
  const benchProvisional =
    !snapshot.bench?.decision || snapshot.bench.decision === "unresolved";
  const flaggedRecords = decisions.filter(
    (decision) => decision.status && decision.status !== "current",
  );
  const staleCount = decisions.filter(
    (decision) =>
      decision.status === "stale_artifact" ||
      decision.status === "missing_artifact",
  ).length;
  return (
    <main className="studio" aria-label="CutRight Studio">
      <header className="titlebar" data-tauri-drag-region>
        <div className="wordmark">
          Cut<span>Right</span>
        </div>
        <div className="project-title">
          {snapshot.manifest.title ??
            snapshot.manifest.project_id ??
            "Untitled"}
          <small>.video-project</small>
        </div>
        <div className="title-actions">
          <button
            aria-label="Toggle theme"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          >
            {theme === "dark" ? "☼" : "◐"}
          </button>
          <button aria-label="Keyboard shortcuts" onClick={() => setHelp(true)}>
            ?
          </button>
        </div>
      </header>
      <div className="shell">
        <aside className="ledger">
          <div className="rail-head">
            <b>SOURCES</b>
            <span>{snapshot.sources.length}</span>
          </div>
          {snapshot.sources.map((source, i) => (
            <button
              key={source.source_id}
              className={`source-row ${i === sourceIndex ? "selected" : ""} ${source.file_present === false ? "missing" : ""}`}
              aria-selected={i === sourceIndex}
              onClick={() => {
                setSourceIndex(i);
                setMode("sources");
              }}
            >
              <span className="poster">▣</span>
              <span className="source-copy">
                <strong>{source.display_name ?? source.source_id}</strong>
                <small>
                  {tc(source.duration_ms)} ·{" "}
                  {source.width ? `${source.width}×${source.height}` : "—"}
                </small>
                <i>
                  {[
                    "ingested",
                    "transcribed",
                    "analyzed",
                    "in_candidates",
                    "in_cut",
                  ].map((key) => (
                    <em
                      title={key}
                      className={source.stages?.[key] ? "on" : ""}
                      key={key}
                    />
                  ))}{" "}
                  {source.is_hdr && <mark>HDR</mark>}
                  {source.integrity && !source.integrity.granted && (
                    <mark
                      className="warn"
                      title={source.integrity.error ?? "not granted for playback"}
                    >
                      BLOCKED
                    </mark>
                  )}
                  {source.integrity?.granted && !source.integrity.verified && (
                    <mark
                      className="warn"
                      title="current bytes do not match the registered hash"
                    >
                      UNVERIFIED
                    </mark>
                  )}
                </i>
              </span>
            </button>
          ))}
          <footer>
            {tc(
              snapshot.sources.reduce(
                (sum, item) => sum + (item.duration_ms ?? 0),
                0,
              ),
            )}
          </footer>
        </aside>
        <section className="viewer">
          <nav className="modes" aria-label="Viewer modes">
            {(["sources", "compare", "finals", "qa"] as Mode[]).map((item) => (
              <button
                key={item}
                disabled={!available[item]}
                aria-disabled={!available[item]}
                title={
                  !available[item]
                    ? `No ${item === "compare" ? "rough cuts yet — run videoctl edit render" : `${item} yet`}`
                    : undefined
                }
                className={mode === item ? "active" : ""}
                onClick={() => setMode(item)}
              >
                {item === "qa" ? "QA" : item}
              </button>
            ))}
          </nav>
          {benchProvisional && (
            <div className="bench-banner" role="status">
              <b>PROVISIONAL REVIEW</b>
              <span>
                timestamp benchmark {snapshot.bench?.decision ?? "missing"} —
                word-edge cuts are unverified
              </span>
            </div>
          )}
          {mode === "sources" && (
            <Sources
              source={selected}
              videoRef={(node) => {
                videoRefs.current.source = node;
              }}
              playing={playing}
              onPlaying={setPlaying}
              playhead={playhead}
              onSeek={seek}
              markers={cutMarkers(undefined, sourceWords)}
            />
          )}
          {mode === "compare" && (
            <Compare
              variants={snapshot.variants}
              variant={variant}
              words={words}
              cursor={cursor.word}
              videoRefs={videoRefs}
              playing={playing}
              onPlaying={setPlaying}
              onSwap={swap}
              onSeek={seek}
              bench={snapshot.bench?.decision === "unresolved"}
              markers={cutMarkers(
                activeVariant?.cut_plan?.segments,
                words[variant] ?? [],
              )}
              flagging={flagging}
              lastFlag={lastFlag}
              onFlag={() => {
                setReasonKind(null);
                setFlagging(true);
              }}
            />
          )}
          {mode === "finals" && (
            <Finals
              finals={snapshot.finals}
              selected={finalPreset}
              onSelect={setFinalPreset}
              selection={selection}
              selecting={selecting}
              onUseFinal={useFinal}
            />
          )}
          {mode === "qa" && (
            <Qa
              report={snapshot.qa}
              acknowledged={qaAcknowledged}
              onAcknowledge={acknowledgeQa}
            />
          )}
          {(mode === "compare" || mode === "finals") && (
            <div className="verdict">
              <div>
                {flagging && mode === "compare" ? (
                  <Reason
                    kind="rejected"
                    title="Flag segment because"
                    reasons={REASONS.segment}
                    note={note}
                    setNote={setNote}
                    commit={commitSegment}
                    onCancel={() => setFlagging(false)}
                  />
                ) : latest ? (
                  <span className={`badge ${latest.verdict}`}>
                    {latest.verdict === "approved" ? "✓" : "✕"} {latest.verdict}{" "}
                    · {latest.reason}
                  </span>
                ) : reasonKind ? (
                  <Reason
                    kind={reasonKind}
                    reasons={REASONS[mode]}
                    note={note}
                    setNote={setNote}
                    commit={commit}
                    onCancel={() => setReasonKind(null)}
                  />
                ) : (
                  <>
                    <button
                      className="approve"
                      onClick={() => setReasonKind("approved")}
                    >
                      ✓ Approve
                    </button>
                    <button
                      className="reject"
                      onClick={() => setReasonKind("rejected")}
                    >
                      ✕ Reject
                    </button>
                  </>
                )}
              </div>
            </div>
          )}
        </section>
        <aside className="inspector">
          {mode === "compare" ? (
            <Transcript
              words={words[variant] ?? []}
              cursor={cursor.word}
              onSeek={seek}
              variant={variant}
            />
          ) : mode === "sources" ? (
            <>
              <SourceFacts source={selected} words={sourceWords} onSeek={seek} />
              <SourceIntegrity
                checks={checks}
                verifying={verifying}
                progress={verifyProgress}
                relinks={relinks}
                onVerify={verifySources}
                onRelink={relink}
              />
            </>
          ) : (
            <div className="empty-rail">
              <b>{mode.toUpperCase()}</b>
              <p>Evidence and verdict context appears here.</p>
            </div>
          )}
        </aside>
      </div>
      <footer className="status-strip">
        <span>
          pipeline {Object.values(snapshot.stages).filter(Boolean).length}/
          {Object.keys(snapshot.stages).length}
        </span>
        <span className={benchProvisional ? "warn" : "good"}>
          ● bench: {snapshot.bench?.decision ?? "unavailable"}
        </span>
        <span className={snapshot.qa?.status === "pass" ? "good" : "warn"}>
          ● QA: {snapshot.qa?.status ?? "pending"}
        </span>
        {artifactIssue(snapshot.qa_artifact) && (
          <span className="warn" title={artifactIssue(snapshot.qa_artifact) ?? ""}>
            ⚠ QA report {artifactIssue(snapshot.qa_artifact)}
          </span>
        )}
        {artifactIssue(snapshot.bench_artifact) && (
          <span className="warn" title={artifactIssue(snapshot.bench_artifact) ?? ""}>
            ⚠ bench report {artifactIssue(snapshot.bench_artifact)}
          </span>
        )}
        {artifactIssue(activeVariant?.cut_plan_artifact) && (
          <span
            className="warn"
            title={artifactIssue(activeVariant?.cut_plan_artifact) ?? ""}
          >
            ⚠ cut plan {artifactIssue(activeVariant?.cut_plan_artifact)}
          </span>
        )}
        <button
          className="strip-toggle"
          aria-expanded={ledgerOpen}
          onClick={() => setLedgerOpen((open) => !open)}
        >
          decisions: session {sessionCount} · total {decisions.length}
          {staleCount > 0 && ` · ${staleCount} stale`}
          {malformedLines.length > 0 &&
            ` · ${malformedLines.length} malformed`}
        </button>
        <button onClick={() => load(snapshot.project_path)}>Refresh</button>
      </footer>
      {ledgerOpen && (
        <DecisionsLedger
          flagged={flaggedRecords}
          malformed={malformedLines}
          close={() => setLedgerOpen(false)}
        />
      )}
      {help && <Help close={() => setHelp(false)} />}
      {palette && (
        <CommandPalette
          modes={available}
          sources={snapshot.sources}
          close={() => setPalette(false)}
          selectMode={setMode}
          selectSource={(index) => {
            setSourceIndex(index);
            setMode("sources");
          }}
          approve={() => setReasonKind("approved")}
          reject={() => setReasonKind("rejected")}
        />
      )}
    </main>
  );
}

function Empty({ onOpen, error }: { onOpen: () => void; error: string }) {
  return (
    <main className="empty-project">
      <div className="wordmark">
        Cut<span>Right</span>
      </div>
      <p>Open a local video project to review its evidence and cuts.</p>
      <button onClick={onOpen}>Open project…</button>
      <code>videoctl project init My.video-project</code>
      {error && <p role="alert">{error}</p>}
    </main>
  );
}
function Sources({
  source,
  videoRef,
  playing,
  onPlaying,
  playhead,
  onSeek,
  markers,
}: {
  source?: Source;
  videoRef: (node: HTMLVideoElement | null) => void;
  playing: boolean;
  onPlaying: (value: boolean) => void;
  playhead: number;
  onSeek: (value: number) => void;
  markers: CutMarker[];
}) {
  if (!source) return <div className="empty-state">No source selected.</div>;
  return (
    <>
      <div className="video-frame">
        {source.path ? (
          <video
            ref={videoRef}
            src={asset(source.path)}
            onPlay={() => onPlaying(true)}
            onPause={() => onPlaying(false)}
            onError={() => onPlaying(false)}
          />
        ) : (
          <div className="video-placeholder">SOURCE PREVIEW</div>
        )}
      </div>
      <Transport
        playing={playing}
        onPlaying={onPlaying}
        playhead={playhead}
        duration={source.duration_ms ?? 0}
        onSeek={onSeek}
        videoKey="source"
        markers={markers}
      />
      <div
        className="waveform"
        role="slider"
        aria-label="Source waveform"
        onClick={(event) =>
          onSeek(
            (event.nativeEvent.offsetX / event.currentTarget.clientWidth) *
              (source.duration_ms ?? 0),
          )
        }
      >
        <i
          style={{
            left: `${Math.min(100, (playhead / (source.duration_ms || 1)) * 100)}%`,
          }}
        />
      </div>
    </>
  );
}
function Compare({
  variants,
  variant,
  words,
  cursor,
  videoRefs,
  playing,
  onPlaying,
  onSwap,
  onSeek,
  bench,
  markers,
  flagging,
  lastFlag,
  onFlag,
}: {
  variants: Variant[];
  variant: string;
  words: Record<string, Word[]>;
  cursor?: Word;
  videoRefs: React.MutableRefObject<Record<string, HTMLVideoElement | null>>;
  playing: boolean;
  onPlaying: (value: boolean) => void;
  onSwap: () => void;
  onSeek: (n: number) => void;
  bench: boolean;
  markers: CutMarker[];
  flagging: boolean;
  lastFlag: { segmentId: string; reason: DecisionReason } | null;
  onFlag: () => void;
}) {
  const active = variants.find((item) => item.id === variant)!;
  return (
    <>
      <div className="compare-head">
        {variants.map((item) => (
          <button
            key={item.id}
            className={`variant-chip ${item.id === variant ? "live" : ""}`}
            onClick={() => item.id !== variant && onSwap()}
          >
            {item.id}
            {bench && <i title="word timestamps unverified" />}
          </button>
        ))}
        <span>word-locked</span>
        {lastFlag && (
          <span className="flag-badge" title="latest segment flag">
            ⚑ {lastFlag.segmentId} · {lastFlag.reason}
          </span>
        )}
        <button
          aria-label="Swap variants"
          onClick={onSwap}
        >
          A ⇄
        </button>
        <button
          className={`flag-segment ${flagging ? "arming" : ""}`}
          title="Flag the segment at the playhead"
          onClick={onFlag}
        >
          ⚑ Flag segment
        </button>
      </div>
      <div className="video-frame compare-videos">
        {variants.map((item) => (
          <video
            key={item.id}
            ref={(node) => {
              videoRefs.current[item.id] = node;
            }}
            src={asset(item.mp4)}
            muted={item.id !== variant}
            preload="metadata"
            className={item.id === variant ? "visible" : "inactive"}
            onPlay={() => onPlaying(true)}
            onPause={() => onPlaying(false)}
          />
        ))}
        <div className="video-placeholder">
          {variant.toUpperCase()} ROUGH CUT
        </div>
      </div>
      <SegmentStrip
        segments={active.cut_plan?.segments ?? []}
        duration={active.duration_ms ?? 0}
        onSeek={onSeek}
      />
      <Transport
        playing={playing}
        onPlaying={onPlaying}
        playhead={cursor?.start_ms ?? 0}
        duration={active.duration_ms ?? 0}
        onSeek={onSeek}
        markers={markers}
      />
    </>
  );
}
function SegmentStrip({
  segments,
  duration,
  onSeek,
}: {
  segments: Segment[];
  duration: number;
  onSeek: (ms: number) => void;
}) {
  return (
    <div className="segments" aria-label="Edit segments">
      {segments.map((segment, i) => (
        <button
          key={segment.id ?? i}
          title={`${segment.id ?? `segment-${i + 1}`} · ${tc((segment.output_end_ms ?? 0) - (segment.output_start_ms ?? 0))}`}
          style={{
            width: `${(((segment.output_end_ms ?? 0) - (segment.output_start_ms ?? 0)) / Math.max(1, duration)) * 100}%`,
          }}
          onClick={() => onSeek(segment.output_start_ms ?? 0)}
        >
          {segment.id ?? `segment-${i + 1}`}
        </button>
      ))}
    </div>
  );
}
function Transport({
  playing,
  onPlaying,
  playhead,
  duration,
  onSeek,
  videoKey,
  markers = [],
}: {
  playing: boolean;
  onPlaying: (x: boolean) => void;
  playhead: number;
  duration: number;
  onSeek: (x: number) => void;
  videoKey?: string;
  markers?: CutMarker[];
}) {
  return (
    <div className="transport">
      <button
        aria-label={playing ? "Pause" : "Play"}
        onClick={() => {
          const video = document.querySelector<HTMLVideoElement>(
            videoKey === "source"
              ? ".video-frame video"
              : ".compare-videos video.visible",
          );
          if (playing) video?.pause();
          else video?.play().catch(() => onPlaying(false));
          onPlaying(!playing);
        }}
      >
        {playing ? "Ⅱ" : "▶"}
      </button>
      <code>
        {tc(playhead)} / {tc(duration)}
      </code>
      <div className="scrub">
        <input
          aria-label="Scrub"
          type="range"
          min="0"
          max={duration}
          value={Math.min(playhead, duration)}
          onChange={(event) => onSeek(Number(event.target.value))}
        />
        {markers.map((marker) => (
          <i
            key={`${marker.ms}-${marker.label}`}
            className="cut-marker"
            title={`${marker.label} · ${tc(marker.ms)}`}
            style={{
              left: `${Math.min(100, (marker.ms / Math.max(1, duration)) * 100)}%`,
            }}
          />
        ))}
      </div>
    </div>
  );
}
function Finals({
  finals,
  selected,
  onSelect,
  selection,
  selecting,
  onUseFinal,
}: {
  finals: Snapshot["finals"];
  selected: string;
  onSelect: (preset: string) => void;
  selection: VariantSelection | null;
  selecting: string | null;
  onUseFinal: (preset: string) => void;
}) {
  return (
    <div className="finals">
      {finals.map((final) => {
        const chosen = selection?.variant === final.preset;
        return (
          <article
            className={`final-card ${final.aspect === "9:16" ? "vertical" : ""} ${final.preset === selected ? "selected" : ""} ${chosen ? "chosen" : ""}`}
            key={final.preset}
          >
            <div className="final-frame">
              {final.mp4 ? (
                <video src={asset(final.mp4)} controls />
              ) : (
                <span>{final.aspect}</span>
              )}
            </div>
            <div className="final-head">
              <b>{final.preset}</b>
              <button
                aria-pressed={final.preset === selected}
                onClick={() => onSelect(final.preset)}
              >
                {final.preset === selected ? "Reviewing" : "Review"}
              </button>
            </div>
            <code>
              {final.width}×{final.height} · {tc(final.duration_ms)}
            </code>
            <button
              className="use-final"
              disabled={selecting === final.preset}
              aria-pressed={chosen}
              onClick={() => onUseFinal(final.preset)}
            >
              {selecting === final.preset
                ? "Binding…"
                : chosen
                  ? "✓ Base for final"
                  : "Use for final"}
            </button>
            {chosen && selection && (
              <code
                className="selection-hash"
                title={`${selection.rough_cut_path} · ${selection.rough_cut_blake3} · ${selection.rough_cut_size} B · ${selection.selected_by}`}
              >
                {selection.rough_cut_blake3.slice(0, 22)}… ·{" "}
                {selection.selected_at.slice(0, 19).replace("T", " ")}
              </code>
            )}
          </article>
        );
      })}
    </div>
  );
}
function Qa({
  report,
  acknowledged,
  onAcknowledge,
}: {
  report?: Snapshot["qa"];
  acknowledged: boolean;
  onAcknowledge: () => void;
}) {
  return (
    <div className="qa-report">
      <h2>{report?.status === "pass" ? "✓ QA passed" : "QA pending"}</h2>
      {report?.checks?.map((check) => (
        <div key={check.id}>
          <span className={check.status === "pass" ? "good" : "bad"}>
            {check.status === "pass" ? "✓" : "✕"}
          </span>
          <b>{check.id}</b>
          <small>{check.evidence}</small>
        </div>
      ))}
      <button
        className="approve"
        disabled={acknowledged}
        onClick={onAcknowledge}
      >
        {acknowledged ? "✓ QA acknowledged" : "Acknowledge QA report"}
      </button>
    </div>
  );
}
function Transcript({
  words,
  cursor,
  onSeek,
  variant,
}: {
  words: Word[];
  cursor?: Word;
  onSeek: (x: number) => void;
  variant: string;
}) {
  return (
    <div className="transcript">
      <div className="rail-head">
        <b>TRANSCRIPT</b>
        <span>{variant}</span>
      </div>
      <p>
        {words.map((word) => (
          <button
            className={word.id === cursor?.id ? "current" : ""}
            key={word.id}
            onClick={() => onSeek(word.start_ms)}
          >
            {word.text}{" "}
          </button>
        ))}
      </p>
      <code>
        {cursor?.source_word_id ?? "preroll"}
        <br />
        {cursor ? `${cursor.start_ms}–${cursor.end_ms}ms` : "No word at cursor"}
      </code>
    </div>
  );
}
function SourceFacts({
  source,
  words,
  onSeek,
}: {
  source?: Source;
  words: Word[];
  onSeek: (ms: number) => void;
}) {
  return (
    <div className="source-facts">
      <b>SOURCE FACTS</b>
      <p>{source?.display_name}</p>
      <code>
        {source?.source_id}
        <br />
        {source?.path ?? "Media path unavailable"}
      </code>
      <div className="source-transcript">
        <b>TRANSCRIPT</b>
        {source?.transcript ? (
          <p>
            {words.map((word) => (
              <button key={word.id} onClick={() => onSeek(word.start_ms)}>
                {word.text}{" "}
              </button>
            ))}
          </p>
        ) : (
          <small>Transcript not available</small>
        )}
      </div>
    </div>
  );
}
function Reason({
  kind,
  title,
  reasons,
  note,
  setNote,
  commit,
  onCancel,
}: {
  kind: "approved" | "rejected";
  title?: string;
  reasons: DecisionReason[];
  note: string;
  setNote: (x: string) => void;
  commit: (x: DecisionReason) => void;
  onCancel?: () => void;
}) {
  return (
    <div className="reason-row">
      <span>
        {title ?? (kind === "approved" ? "Approve because" : "Reject because")}
      </span>
      {reasons.map((reason) => (
        <button
          key={reason}
          onClick={() => (reason === "other" ? undefined : commit(reason))}
        >
          {reason.replaceAll("_", " ")}
        </button>
      ))}
      <input
        aria-label="Other reason"
        maxLength={200}
        value={note}
        onChange={(event) => setNote(event.target.value)}
        placeholder="Other reason"
      />
      <button disabled={!note.trim()} onClick={() => commit("other")}>
        Save
      </button>
      {onCancel && (
        <button className="reason-cancel" onClick={onCancel}>
          cancel
        </button>
      )}
    </div>
  );
}
const shortHash = (value: string) =>
  value.length > 20 ? `${value.slice(0, 20)}…` : value;
function SourceIntegrity({
  checks,
  verifying,
  progress,
  relinks,
  onVerify,
  onRelink,
}: {
  checks: SourceCheck[] | null;
  verifying: boolean;
  progress: string;
  relinks: Record<string, RelinkResult>;
  onVerify: () => void;
  onRelink: (sourceId: string) => void;
}) {
  const failed = checks?.filter((check) => !check.matches).length ?? 0;
  return (
    <div className="source-integrity">
      <div className="rail-head">
        <b>SOURCE INTEGRITY</b>
        {verifying && <span className="warn">{progress || "hashing…"}</span>}
      </div>
      <button className="verify-btn" onClick={onVerify} disabled={verifying}>
        {verifying ? `Verifying ${progress}` : "Verify sources"}
      </button>
      {checks && (
        <p className={`verify-summary ${failed ? "bad" : "good"}`}>
          {failed
            ? `${failed} of ${checks.length} sources failed verification`
            : `all ${checks.length} sources match the manifest`}
        </p>
      )}
      <ul className="verify-list">
        {(checks ?? []).map((check, index) => (
          <li
            key={check.source_id}
            className={`verify-row ${check.matches ? "pass" : "fail"}`}
            style={{ animationDelay: `${index * 45}ms` }}
          >
            <span className="verify-state" aria-hidden="true">
              {check.matches ? "✓" : "✕"}
            </span>
            <code className="verify-id">{check.source_id}</code>
            <code
              className="verify-hash"
              title={`expected ${check.expected_blake3}\nactual ${check.actual_blake3 ?? check.error ?? "unreadable"}`}
            >
              exp {shortHash(check.expected_blake3)}
              <br />
              act {shortHash(check.actual_blake3 ?? check.error ?? "missing")}
            </code>
            {!check.matches && (
              <button className="relink-btn" onClick={() => onRelink(check.source_id)}>
                Relink…
              </button>
            )}
            {relinks[check.source_id] && (
              <small className="relink-note">
                relinked · {shortHash(relinks[check.source_id].blake3)} ·{" "}
                {relinks[check.source_id].matches ? "match" : "still mismatched"}
              </small>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
function DecisionsLedger({
  flagged,
  malformed,
  close,
}: {
  flagged: DecisionRecord[];
  malformed: DecisionReplay["malformed_lines"];
  close: () => void;
}) {
  return (
    <div className="decisions-panel" role="dialog" aria-label="Decision ledger">
      <div className="panel-head">
        <b>DECISION LEDGER</b>
        <button aria-label="Close ledger" onClick={close}>
          ×
        </button>
      </div>
      {flagged.length === 0 && malformed.length === 0 ? (
        <p className="panel-empty">All records current.</p>
      ) : (
        <ul>
          {flagged.map((record) => (
            <li key={record.decision_id}>
              <span className={`status-chip ${record.status}`}>
                {record.status}
              </span>
              <code>
                {record.kind} · {record.subject}
              </code>
              <small>
                {record.verdict} · {record.reason} · {record.ts}
              </small>
            </li>
          ))}
        </ul>
      )}
      {malformed.length > 0 && (
        <p className="malformed">
          {malformed.length} malformed line
          {malformed.length === 1 ? "" : "s"}:{" "}
          {malformed
            .map((line) => `#${line.line_number} (${line.error})`)
            .join(", ")}
        </p>
      )}
    </div>
  );
}
function Help({ close }: { close: () => void }) {
  return (
    <div
      className="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Keyboard shortcuts"
    >
      <div>
        <button aria-label="Close shortcuts" onClick={close}>
          ×
        </button>
        <h2>Keyboard shortcuts</h2>
        <p>
          Space play/pause · J/K/L seek, pause, play/rate · A swap variant · ← → words · ⇧← → segments · ,/. frames
        </p>
        <p>⌘1–4 modes · ⌘K commands/sources · ⌘R refresh · 1–9 sources · Esc close</p>
      </div>
    </div>
  );
}
function CommandPalette({
  modes,
  sources,
  close,
  selectMode,
  selectSource,
  approve,
  reject,
}: {
  modes: Record<Mode, boolean>;
  sources: Source[];
  close: () => void;
  selectMode: (mode: Mode) => void;
  selectSource: (index: number) => void;
  approve: () => void;
  reject: () => void;
}) {
  return (
    <div className="dialog command-palette" role="dialog" aria-modal="true" aria-label="Command palette">
      <div>
        <button aria-label="Close commands" onClick={close}>×</button>
        <h2>Commands</h2>
        <div className="command-list">
          {(["sources", "compare", "finals", "qa"] as Mode[]).map((mode) => (
            <button key={mode} disabled={!modes[mode]} onClick={() => { selectMode(mode); close(); }}>Switch to {mode === "qa" ? "QA" : mode}</button>
          ))}
          <button onClick={() => { approve(); close(); }}>Approve current review</button>
          <button onClick={() => { reject(); close(); }}>Reject current review</button>
          {sources.map((source, index) => (
            <button key={source.source_id} onClick={() => { selectSource(index); close(); }}>Source {index + 1}: {source.display_name ?? source.source_id}</button>
          ))}
        </div>
      </div>
    </div>
  );
}
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
