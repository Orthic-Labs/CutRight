import { useEffect, useMemo, useRef, useState } from "react";
import { swapTarget } from "./word-lock";
import { cutMarkers } from "./cut-markers";
import { selectLedgerView } from "./decision-selectors";
import { useProject } from "./hooks/useProject";
import { usePlayback } from "./hooks/usePlayback";
import { useReviewLedger } from "./hooks/useReviewLedger";
import { useKeyboard } from "./hooks/useKeyboard";
import { qa as qaMode } from "./lib/api";
import { Empty } from "./components/Empty";
import { Transcript } from "./components/Transcript";
import { SourceFacts } from "./components/SourceFacts";
import { SourceIntegrityPanel } from "./components/SourceIntegrityPanel";
import { DecisionsLedger } from "./components/DecisionsLedger";
import { Help } from "./components/Help";
import { CommandPalette } from "./components/CommandPalette";
import { TitleBar } from "./components/TitleBar";
import { ModeRail } from "./components/ModeRail";
import { SourcesRail } from "./components/SourcesRail";
import { VerdictPanel } from "./components/VerdictPanel";
import { StatusStrip } from "./components/StatusStrip";
import { SourcesMode } from "./modes/SourcesMode";
import { CompareMode } from "./modes/CompareMode";
import { FinalsMode } from "./modes/FinalsMode";
import { QaMode } from "./modes/QaMode";
import { SettingsMode } from "./modes/SettingsMode";
import type { Register } from "./types";

export function App() {
  const [help, setHelp] = useState(false);
  const [palette, setPalette] = useState(false);
  const [theme, setTheme] = useState(
    () =>
      localStorage.getItem("cutright-theme") ??
      (matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark"),
  );
  // Three parked register variants (redesign spec Phase 2), QA-only picker
  // — see TitleBar/RegisterSwitch. R1 is the default until Adrian locks one
  // into brands.md; a real user never sees the switcher (gated on `qaMode`
  // from lib/api, the same flag the browser-QA fixture path uses).
  const [register, setRegister] = useState<Register>(
    () =>
      (new URLSearchParams(location.search).get("register") as Register) ||
      "cutting-room",
  );
  // Last swap's word-cut delta (word-lock.ts's `swapTarget().cut_count`),
  // shown at the bench switch's lock point — spec Phase 1's "3 words cut
  // here" evidence. `null` until the first swap in this session.
  const [lastDelta, setLastDelta] = useState<number | null>(null);

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
    playheadRef,
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
    playheadRef,
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

  useEffect(() => {
    document.documentElement.dataset.register = register;
  }, [register]);

  function swap() {
    const other = variant === "natural" ? "tight" : "natural";
    const target = swapTarget(
      words[variant] ?? [],
      words[other] ?? [],
      playheadRef.current,
    );
    if (target.refused) {
      setError(`${other} has no content`);
      return;
    }
    setLastDelta(target.cut_count);
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
    playheadRef,
    seek,
    togglePlayback,
    pause,
    playOrIncreaseRate,
    swap,
    seekWord,
    seekSegment,
    frameDuration,
  });

  // Perf fix (REV2 audit): a reversed-copy `.find()` plus two separate
  // `.filter()` passes over `decisions` used to run on every render.
  // Collapsed into one selector, memoized on the inputs that actually
  // change it. See decision-selectors.ts.
  const { latest, flaggedRecords, staleCount } = useMemo(
    () => selectLedgerView(decisions, mode, variant, finalPreset),
    [decisions, mode, variant, finalPreset],
  );

  if (!snapshot) return <Empty onOpen={() => load()} error={error} />;
  const qaAcknowledged = decisions.some(
    (decision) => decision.kind === "qa_ack",
  );
  const benchProvisional =
    !snapshot.bench?.decision || snapshot.bench.decision === "unresolved";
  return (
    <main className="studio" aria-label="CutRight Studio">
      <TitleBar
        title={
          snapshot.manifest.title ?? snapshot.manifest.project_id ?? "Untitled"
        }
        theme={theme}
        onToggleTheme={() => setTheme(theme === "dark" ? "light" : "dark")}
        onHelp={() => setHelp(true)}
        qa={qaMode}
        register={register}
        setRegister={setRegister}
      />
      <div className="shell">
        <ModeRail mode={mode} setMode={setMode} available={available} />
        <SourcesRail
          sources={snapshot.sources}
          sourceIndex={sourceIndex}
          onSelect={(index) => {
            setSourceIndex(index);
            setMode("sources");
          }}
        />
        <section className="viewer">
          {benchProvisional && (
            <div className="bench-banner" role="status">
              <b>PROVISIONAL REVIEW</b>
              <span>
                timestamp benchmark {snapshot.bench?.decision ?? "missing"} —
                word-edge cuts are unverified
              </span>
            </div>
          )}
          {/* Mode switch motion (spec: "100ms opacity, no slide — modes are
              places, not slides"). Remounting on `key={mode}` fires the CSS
              `mode-in` animation on every mode change; reduced-motion zeroes
              it globally (styles.css). */}
          <div key={mode} className="mode-panel">
            {mode === "sources" && (
              <SourcesMode
                source={selected}
                videoRef={(node) => {
                  videoRefs.current.source = node;
                }}
                playing={playing}
                onPlaying={setPlaying}
                playhead={playhead}
                playheadRef={playheadRef}
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
                delta={lastDelta}
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
            {mode === "settings" && <SettingsMode project={snapshot} />}
            {(mode === "compare" || mode === "finals") && (
              <VerdictPanel
                mode={mode}
                flagging={flagging}
                note={note}
                setNote={setNote}
                commitSegment={commitSegment}
                onCancelFlag={() => setFlagging(false)}
                latest={latest}
                reasonKind={reasonKind}
                commit={commit}
                onCancelReason={() => setReasonKind(null)}
                onApprove={() => setReasonKind("approved")}
                onReject={() => setReasonKind("rejected")}
              />
            )}
          </div>
        </section>
        <aside className="inspector">
          <div key={mode} className="mode-panel">
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
          </div>
        </aside>
      </div>
      <StatusStrip
        stages={snapshot.stages}
        bench={snapshot.bench}
        benchProvisional={benchProvisional}
        qa={snapshot.qa}
        qaArtifact={snapshot.qa_artifact}
        benchArtifact={snapshot.bench_artifact}
        cutPlanArtifact={activeVariant?.cut_plan_artifact}
        ledgerOpen={ledgerOpen}
        onToggleLedger={() => setLedgerOpen((open) => !open)}
        sessionCount={sessionCount}
        totalDecisions={decisions.length}
        staleCount={staleCount}
        malformedCount={malformedLines.length}
        onRefresh={() => load(snapshot.project_path)}
      />
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
