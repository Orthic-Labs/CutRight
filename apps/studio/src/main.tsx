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
import "./styles.css";

type Mode = "sources" | "compare" | "finals" | "qa";
type Variant = {
  id: string;
  mp4?: string | null;
  fps?: number;
  duration_ms?: number;
  cut_plan?: { segments?: Segment[] } | null;
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
};
type Snapshot = {
  project_path: string;
  generated_at: string;
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
  bench?: { decision?: string };
  decisions_path?: string;
};
type Decision = {
  kind: string;
  verdict?: string | null;
  reason: string;
  subject: string;
  variant?: string | null;
  segment_id?: string | null;
  ts: string;
  snapshot_generated_at?: string;
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
    id: `ow_${i}`,
    text,
    start_ms: i * 620,
    end_ms: i * 620 + 510,
    source_word_id: `source-a:w_${i}`,
  })),
  tight: ["This", "is", "the", "same", "spoken", "sentence", "breathe"].map(
    (text, i) => ({
      id: `tw_${i}`,
      text,
      start_ms: i * 490,
      end_ms: i * 490 + 410,
      source_word_id: `source-a:w_${i < 6 ? i : 9}`,
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

const memoryDecisions: Decision[] = [];
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
    return { decisions: memoryDecisions, skipped: 0 } as T;
  if (command === "append_decision") {
    memoryDecisions.push(args.decision as Decision);
    return undefined as T;
  }
  if (command === "verify_sources")
    return fixture.sources.map((source) => ({
      source_id: source.source_id,
      matches: true,
    })) as T;
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
  const [decisions, setDecisions] = useState<Decision[]>([]);
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
      localStorage.setItem("cutright-project", picked);
      const initial: Mode = next.finals.length
        ? "finals"
        : next.variants.length
          ? "compare"
          : "sources";
      setMode((localStorage.getItem("cutright-mode") as Mode) ?? initial);
      const history = await call<{ decisions: Decision[] }>("read_decisions", {
        path: picked,
      });
      setDecisions(history.decisions);
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
        if (event.key === "Escape") setReasonKind(null);
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
      }
      if (event.key === " " && mode !== "qa") {
        event.preventDefault();
        togglePlayback();
      }
      if (event.key.toLowerCase() === "j") seek(playhead - 2000);
      if (event.key.toLowerCase() === "k") pause();
      if (event.key.toLowerCase() === "l") playOrIncreaseRate();
      if (event.key.toLowerCase() === "a" && mode === "compare") swap();
      if (event.key.toLowerCase() === "y") setReasonKind("approved");
      if (event.key.toLowerCase() === "x") setReasonKind("rejected");
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
  async function commit(reason: string) {
    if (!snapshot || !reasonKind) return;
    const subject =
      mode === "finals"
        ? (snapshot.finals[0]?.mp4 ?? "render/final.mp4")
        : (activeVariant?.mp4 ?? `render/rough-cuts/${variant}.mp4`);
    const decision: Decision = {
      kind: mode === "finals" ? "final_verdict" : "variant_verdict",
      verdict: reasonKind,
      reason,
      subject,
      variant: mode === "finals" ? null : variant,
      ts: new Date().toISOString(),
      snapshot_generated_at: snapshot.generated_at,
    };
    await call("append_decision", { path: snapshot.project_path, decision });
    setDecisions((old) => [...old, decision]);
    setReasonKind(null);
    setNote("");
  }
  if (!snapshot) return <Empty onOpen={() => load()} error={error} />;
  const latest = [...decisions]
    .reverse()
    .find(
      (decision) =>
        decision.variant === variant ||
        (mode === "finals" && decision.kind === "final_verdict"),
    );
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
            />
          )}
          {mode === "finals" && <Finals finals={snapshot.finals} />}
          {mode === "qa" && <Qa report={snapshot.qa} />}
          {mode !== "qa" && (
            <div className="verdict">
              <div>
                {latest ? (
                  <span className={`badge ${latest.verdict}`}>
                    {latest.verdict === "approved" ? "✓" : "✕"} {latest.verdict}{" "}
                    · {latest.reason}
                  </span>
                ) : reasonKind ? (
                  <Reason
                    kind={reasonKind}
                    note={note}
                    setNote={setNote}
                    commit={commit}
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
            <SourceFacts source={selected} words={sourceWords} onSeek={seek} />
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
        <span
          className={
            snapshot.bench?.decision === "unresolved" ? "warn" : "good"
          }
        >
          ● bench: {snapshot.bench?.decision ?? "unavailable"}
        </span>
        <span className={snapshot.qa?.status === "pass" ? "good" : "warn"}>
          ● QA: {snapshot.qa?.status ?? "pending"}
        </span>
        <span>
          decisions: this session {decisions.length} · total {decisions.length}
        </span>
        <button onClick={() => load(snapshot.project_path)}>Refresh</button>
      </footer>
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
}: {
  source?: Source;
  videoRef: (node: HTMLVideoElement | null) => void;
  playing: boolean;
  onPlaying: (value: boolean) => void;
  playhead: number;
  onSeek: (value: number) => void;
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
        <button aria-label="Swap variants" onClick={onSwap}>
          A ⇄
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
}: {
  playing: boolean;
  onPlaying: (x: boolean) => void;
  playhead: number;
  duration: number;
  onSeek: (x: number) => void;
  videoKey?: string;
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
      <input
        aria-label="Scrub"
        type="range"
        min="0"
        max={duration}
        value={Math.min(playhead, duration)}
        onChange={(event) => onSeek(Number(event.target.value))}
      />
    </div>
  );
}
function Finals({ finals }: { finals: Snapshot["finals"] }) {
  return (
    <div className="finals">
      {finals.map((final) => (
        <article
          className={`final-card ${final.aspect === "9:16" ? "vertical" : ""}`}
          key={final.preset}
        >
          <div className="final-frame">
            {final.mp4 ? (
              <video src={asset(final.mp4)} controls />
            ) : (
              <span>{final.aspect}</span>
            )}
          </div>
          <b>{final.preset}</b>
          <code>
            {final.width}×{final.height} · {tc(final.duration_ms)}
          </code>
        </article>
      ))}
    </div>
  );
}
function Qa({ report }: { report?: Snapshot["qa"] }) {
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
  note,
  setNote,
  commit,
}: {
  kind: "approved" | "rejected";
  note: string;
  setNote: (x: string) => void;
  commit: (x: string) => void;
}) {
  const reasons =
    kind === "approved"
      ? ["pacing", "word_edges", "energy", "length", "other"]
      : [
          "clipped_word",
          "too_tight",
          "too_loose",
          "bad_boundary",
          "wrong_take",
          "other",
        ];
  return (
    <div className="reason-row">
      <span>{kind === "approved" ? "Approve because" : "Reject because"}</span>
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
