import { useEffect, useRef, useState } from "react";
import { swapTarget } from "./word-lock";
import { cutMarkers } from "./cut-markers";
import { REASONS } from "./contracts/review";
import { useProject } from "./hooks/useProject";
import { usePlayback } from "./hooks/usePlayback";
import { useReviewLedger } from "./hooks/useReviewLedger";
import { useKeyboard } from "./hooks/useKeyboard";
import { Empty } from "./components/Empty";
import { Reason } from "./components/Reason";
import { Transcript } from "./components/Transcript";
import { SourceFacts } from "./components/SourceFacts";
import { SourceIntegrityPanel } from "./components/SourceIntegrityPanel";
import { DecisionsLedger } from "./components/DecisionsLedger";
import { Help } from "./components/Help";
import { CommandPalette } from "./components/CommandPalette";
import { SourcesMode } from "./modes/SourcesMode";
import { CompareMode } from "./modes/CompareMode";
import { FinalsMode } from "./modes/FinalsMode";
import { QaMode } from "./modes/QaMode";
import { artifactIssue } from "./types";
import type { Mode } from "./types";
import { tc } from "./lib/api";

export function App() {
  const [help, setHelp] = useState(false);
  const [palette, setPalette] = useState(false);
  const [theme, setTheme] = useState(
    () =>
      localStorage.getItem("cutright-theme") ??
      (matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark"),
  );

  // `useProject.load()` needs to seed `useReviewLedger`'s state, but ledger
  // is composed after project below; a render-phase ref keeps `load()`'s
  // callbacks always pointed at the latest ledger setters without changing
  // when `load` itself is (re)created. See hooks/useProject.ts.
  const ledgerRef = useRef<ReturnType<typeof useReviewLedger> | null>(null);
  const project = useProject({
    setDecisions: (records) => ledgerRef.current?.setDecisions(records),
    setMalformedLines: (lines) => ledgerRef.current?.setMalformedLines(lines),
    setSelection: (selection) => ledgerRef.current?.setSelection(selection),
    setFinalPreset: (updater) => ledgerRef.current?.setFinalPreset(updater),
  });
  const {
    snapshot,
    mode,
    setMode,
    sourceIndex,
    setSourceIndex,
    variant,
    setVariant,
    words,
    sourceWords,
    error,
    setError,
    selected,
    available,
    load,
  } = project;
  const activeVariant = snapshot?.variants.find((item) => item.id === variant);

  const playback = usePlayback({ mode, variant, words, activeVariant });
  const {
    playhead,
    setPlayhead,
    playing,
    setPlaying,
    videoRefs,
    cursor,
    seek,
    togglePlayback,
    pause,
    playOrIncreaseRate,
    frameDuration,
    seekWord,
    seekSegment,
  } = playback;

  const ledger = useReviewLedger({
    snapshot,
    mode,
    variant,
    cursor,
    playhead,
    activeVariant,
    setError,
  });
  ledgerRef.current = ledger;
  const {
    reasonKind,
    setReasonKind,
    note,
    setNote,
    decisions,
    malformedLines,
    sessionCount,
    ledgerOpen,
    setLedgerOpen,
    checks,
    verifying,
    verifyProgress,
    relinks,
    selection,
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
  } = ledger;

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("cutright-theme", theme);
  }, [theme]);

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

  useKeyboard({
    active: Boolean(snapshot),
    mode,
    setMode,
    available,
    setPalette,
    setHelp,
    setReasonKind,
    setFlagging,
    setLedgerOpen,
    setSourceIndex,
    sourceCount: snapshot?.sources.length ?? 0,
    reload: () => {
      if (snapshot) void load(snapshot.project_path);
    },
    playhead,
    seek,
    togglePlayback,
    pause,
    playOrIncreaseRate,
    swap,
    seekWord,
    seekSegment,
    frameDuration,
  });

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
            <SourcesMode
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
            <CompareMode
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
            <FinalsMode
              finals={snapshot.finals}
              selected={finalPreset}
              onSelect={setFinalPreset}
              selection={selection}
              selecting={selecting}
              onUseFinal={useFinal}
            />
          )}
          {mode === "qa" && (
            <QaMode
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
              <SourceIntegrityPanel
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
