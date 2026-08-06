// Finalize — capture -> unlock -> commit (Sol audit 2026-07-16, finding F3).
//
// The original `finalize_transcript_with_audio_secs` ran entirely under the
// caller's `EngineRuntime` mutex hold: stop-time UIA/AX refresh, local +
// cloud (L1/L2/L3) polish HTTP (up to the l3_cleanup hard cap), and OS-level
// delivery/command dispatch all happened while the lock was held, so
// `CancelDictation`/`GetState`/a new `StartDictation` queued behind whatever
// finalize happened to be running.
//
// The fix splits the work into three phases:
//   1. `finalize_phase1_capture` (locked, brief) — validates the stale
//      guard, resolves the cheap/pure control-command and ai-transform-tail
//      parsing, and snapshots everything phase 2 needs into an owned
//      `FinalizeCapture`. Short-circuit outcomes (stale no-op, transcribe
//      error, `zephyr cancel`) are resolved here and returned as `Done`.
//   2. `finalize_phase2_process` (a free function — NOT a method, so it
//      cannot accidentally borrow the runtime) — runs with no runtime lock
//      held at all: the stop-time field-context refresh, local + cloud
//      polish, standalone-command dispatch, and delivery. Polled via
//      `is_current` at the natural step boundaries (classify -> dispatch,
//      context -> polish, polish -> deliver) so a Cancel or a new Start that
//      raced ahead stops this finalize short of any further OS-visible
//      action.
//   3. `finalize_phase3_commit` (locked, brief) — re-validates the finalize
//      generation and commits `last_delivery` / `recent` / `state` / focus
//      reset, or discards the result if superseded.
//
// `finalize_transcript_with_audio_secs` composes all three back-to-back for
// direct callers (tests, any other `&mut EngineRuntime` caller) — same
// external signature, same observable behavior, since nothing else can run
// concurrently against an owned `&mut self`. `finalize_unlocked` is the
// `Arc<Mutex<EngineRuntime>>`-aware entry point the IPC worker-event-pump
// uses to actually release the lock across phase 2.

/// Everything phase 2 needs, snapshotted under the runtime lock in phase 1.
/// Owned/cloned so phase 2 can run without borrowing `EngineRuntime` at all.
struct FinalizeCapture {
    session_id: String,
    stop_started_at: Option<Instant>,
    transcript: String,
    send_enter: bool,
    ai_transform: Option<heardright_core::text_pipeline::AiTransformIntent>,
    audio_secs: Option<f32>,
    focus: Arc<Mutex<FocusTracker>>,
    last_delivery_text: Option<String>,
    /// `StopKind::CancelToHistory` finalize (engine-side: `pending_local_only`,
    /// mirroring `send_enter`/`pending_send_enter`). Phase 2 runs ASR and L0
    /// local polish to completion exactly like a normal Stop, but skips the
    /// cloud L1/L2/L3 polish and ai-transform (prompt/summarize) lanes so a
    /// cancelled utterance's text is never sent to the cloud.
    /// Named `local_only_stop` (not `local_only`) to avoid colliding with
    /// the unrelated local `local_only` STRING binding phase 2 already uses
    /// for the deterministic L0-polished text (see `finalize_phase2_process`).
    local_only_stop: bool,
}

/// What phase 3 must commit once phase 2 resolves an outcome, IF the
/// finalize generation is still current. Every non-short-circuited exit path
/// of the original function ended with the SAME two mutations (state ->
/// Idle, focus reset), optionally preceded by a `last_delivery` write and
/// (only for the normal in-app delivery path) a `recent` push — this struct
/// carries exactly that, so phase 3 doesn't need to re-inspect the outcome.
struct FinalizeCommit {
    outcome: FinalizeOutcome,
    delivery_record: Option<DeliveryRecord>,
    push_recent: bool,
}

impl FinalizeCommit {
    fn outcome_only(outcome: FinalizeOutcome) -> Self {
        Self {
            outcome,
            delivery_record: None,
            push_recent: false,
        }
    }

    fn with_record(outcome: FinalizeOutcome, record: DeliveryRecord, push_recent: bool) -> Self {
        Self {
            outcome,
            delivery_record: Some(record),
            push_recent,
        }
    }

    /// Superseded by a Cancel or a new Start mid-phase-2 (see `is_current`
    /// checkpoints below). No delivery/dispatch happened past this point;
    /// phase 3's own generation check will independently discard this too,
    /// so this is belt-and-suspenders, not the sole guard.
    fn discard() -> Self {
        Self {
            outcome: FinalizeOutcome::NoOp,
            delivery_record: None,
            push_recent: false,
        }
    }
}

enum FinalizePhase1 {
    /// Already resolved under the phase-1 lock — no phase 2/3 needed.
    Done(Result<FinalizeOutcome, String>),
    /// Proceed to phase 2 with this snapshot; `generation` is the
    /// `EngineRuntime::sequence` value captured at snapshot time, checked
    /// again at phase 3 (and polled mid-phase-2) to detect a superseding
    /// Cancel or new Start.
    Continue {
        capture: FinalizeCapture,
        generation: u64,
    },
}

/// Log-and-return-true helper for the phase-2 supersession checkpoints:
/// true means "stop here, this finalize was superseded" (a Cancel or a new
/// Start bumped the generation counter since phase 1 captured it).
fn discard_if_superseded(
    is_current: &dyn Fn() -> bool,
    session_id: &str,
    step: &'static str,
) -> bool {
    if is_current() {
        false
    } else {
        tracing::info!(
            session_id,
            step,
            "finalize_superseded_skipping_remaining_work"
        );
        true
    }
}

impl EngineRuntime {
    /// Phase 1 (locked, brief) of finalize — see module doc comment.
    fn finalize_phase1_capture(
        &mut self,
        session_id: &str,
        result: Result<String, String>,
        audio_secs: Option<f32>,
    ) -> FinalizePhase1 {
        // Stale guard — if the active session moved on (cancel, new session,
        // watchdog-after-real), drop this final. The shell also enforces this
        // guard; defense in depth.
        let active = self.interaction_is_live(session_id)
            && matches!(&self.state, EngineState::Transcribing { session_id: s } if s == session_id);
        if !active {
            return FinalizePhase1::Done(Ok(FinalizeOutcome::NoOp));
        }
        let stop_started_at = self.stop_started_at.take();
        let send_enter = std::mem::take(&mut self.pending_send_enter);
        let local_only_stop = std::mem::take(&mut self.pending_local_only);
        let mut transcript = match result {
            Ok(t) => t,
            Err(message) => {
                self.record_terminal_tombstone(session_id);
                self.state = EngineState::Error {
                    session_id: session_id.to_string(),
                    message: message.clone(),
                };
                trace_stop_to_engine_outcome(stop_started_at, "error");
                return FinalizePhase1::Done(Err(message));
            }
        };
        // Wake command authority arrives only from Sherpa through
        // `pending_send_enter` or the worker's cancel event. Final main-ASR
        // text remains dictation; it cannot select STOP, SEND, or CANCEL.
        let mut ai_transform = if let Some(command) =
            heardright_core::text_pipeline::parse_ai_transform_command(&transcript)
        {
            tracing::info!(
                intent = ?command.intent,
                "ai_transform_tail_detected"
            );
            transcript = command.clean_text;
            Some(command.intent)
        } else {
            None
        };
        // Field bug 2026-07-16: saying just "summarize" (ASR often adds a
        // leading filler the tail parser strips) matched the TAIL lane with
        // EMPTY content — L3 then summarized nothing and pasted the model's
        // "No content provided to summarize." An empty-content summarize tail
        // IS a bare trigger: normalize it back so phase 2's selection lanes
        // (captured UIA selection, then the Ctrl+C copy-fallback) handle it.
        if matches!(
            ai_transform,
            Some(heardright_core::text_pipeline::AiTransformIntent::Summarize)
        ) && transcript.trim().is_empty()
        {
            tracing::info!("ai_transform_empty_tail_treated_as_bare_trigger");
            transcript = "summarize".to_string();
            ai_transform = None;
        }
        let last_delivery_text = self.last_delivery.as_ref().map(|r| r.transcript.clone());
        let generation = self.sequence;
        FinalizePhase1::Continue {
            capture: FinalizeCapture {
                session_id: session_id.to_string(),
                stop_started_at,
                transcript,
                send_enter,
                ai_transform,
                audio_secs,
                focus: self.focus.clone(),
                last_delivery_text,
                local_only_stop,
            },
            generation,
        }
    }

    /// Phase 3 (locked, brief) of finalize — see module doc comment. Applies
    /// `commit` only if `expected_generation` still matches the live counter
    /// AND the runtime is still `Transcribing` this same session — either
    /// mismatching means a Cancel or a new Start raced ahead while phase 2
    /// ran unlocked, so this result is discarded instead of clobbering
    /// whatever is running now.
    fn finalize_phase3_commit(
        &mut self,
        session_id: &str,
        expected_generation: u64,
        commit: FinalizeCommit,
    ) -> FinalizeOutcome {
        let current = self.sequence == expected_generation
            && self.interaction_is_live(session_id)
            && matches!(&self.state, EngineState::Transcribing { session_id: s } if s == session_id);
        if !current {
            tracing::warn!(
                session_id,
                expected_generation,
                current_generation = self.sequence,
                "finalize_commit_discarded_stale_generation"
            );
            return FinalizeOutcome::NoOp;
        }
        if let Some(record) = commit.delivery_record {
            self.last_delivery = Some(record.clone());
            if commit.push_recent {
                self.push_recent(record);
            }
        }
        self.record_terminal_tombstone(session_id);
        self.state = EngineState::Idle;
        self.focus.lock().reset();
        commit.outcome
    }

    /// Apply a finalized transcript and return the sidecar-owned outcome for
    /// the shell to mirror into its UI state/history. Composes phase 1 + 2 +
    /// 3 back-to-back — for a direct `&mut EngineRuntime` caller (tests, any
    /// non-IPC caller) there is no concurrent lock holder to release the
    /// mutex FOR, so this stays a single synchronous call with identical
    /// external behavior to before the F3 split.
    pub fn finalize_transcript_with_audio_secs(
        &mut self,
        session_id: &str,
        result: Result<String, String>,
        audio_secs: Option<f32>,
    ) -> Result<FinalizeOutcome, String> {
        let (capture, generation) =
            match self.finalize_phase1_capture(session_id, result, audio_secs) {
                FinalizePhase1::Done(result) => return result,
                FinalizePhase1::Continue {
                    capture,
                    generation,
                } => (capture, generation),
            };
        let commit = finalize_phase2_process(capture, &|| true)?;
        Ok(self.finalize_phase3_commit(session_id, generation, commit))
    }
}

/// Run finalize using the capture -> unlock -> commit split (finding F3).
/// Phase 1 — and, for the `AutoStop` worker event, the atomic `begin_stop`
/// that must share its lock acquisition with phase 1's stale-check — runs
/// under a BRIEF runtime-mutex hold. Phase 2 (stop-time UIA/AX refresh,
/// local + cloud L1/L2/L3 polish, delivery/standalone-command dispatch) runs
/// with NO runtime lock held at all, so `CancelDictation`/`GetState`/a new
/// `StartDictation` are never blocked behind an in-flight cloud-polish HTTP
/// call or an OS paste/dispatch. Phase 3 re-acquires the lock only long
/// enough to verify the finalize generation is still current and commit; on
/// a mismatch the result is discarded (`Ok(FinalizeOutcome::NoOp)`) instead
/// of clobbering whatever ran after this finalize was captured.
///
/// `begin_stop_send_enter` is `Some(send_enter)` only for the final `AutoStop`
/// worker event. Its earlier `AutoStopBegin` normally moved
/// `Recording -> Transcribing` before final ASR; this idempotent call closes
/// the race if final result reaches pump before begin signal. Manual-stop
/// `TranscriptFinal` uses `None` because `StopDictation` already began stop.
pub(crate) fn finalize_unlocked(
    runtime: &Arc<Mutex<EngineRuntime>>,
    session_id: &str,
    result: Result<String, String>,
    audio_secs: Option<f32>,
    begin_stop_send_enter: Option<bool>,
) -> Result<FinalizeOutcome, String> {
    let phase1 = {
        let mut rt = runtime.lock();
        if let Some(send_enter) = begin_stop_send_enter {
            // AutoStop (hands-free zephyr stop/send tail) never carries the
            // cancel-to-history intent — that only originates from the
            // shell's explicit `Request::StopDictation { local_only: true }`
            // dispatch, which calls `begin_stop` directly (see
            // `ipc_sections/section02.rs`).
            let _ = rt.begin_stop(session_id, send_enter, false);
        }
        rt.finalize_phase1_capture(session_id, result, audio_secs)
    };
    let (capture, generation) = match phase1 {
        FinalizePhase1::Done(result) => return result,
        FinalizePhase1::Continue {
            capture,
            generation,
        } => (capture, generation),
    };
    let check_runtime = runtime.clone();
    let is_current = move || check_runtime.lock().sequence_matches(generation);
    let commit = finalize_phase2_process(capture, &is_current)?;
    let mut rt = runtime.lock();
    Ok(rt.finalize_phase3_commit(session_id, generation, commit))
}

/// Phase 2 (UNLOCKED) of finalize — see module doc comment. A free function,
/// not a method: it must not be able to borrow `EngineRuntime`, since the
/// whole point is that no runtime lock is held while this runs. Everything
/// it needs comes from `capture`; everything it calls is a free function
/// (text_polish / l3_cleanup / command_classify / command_dispatch / focus /
/// delivery) or a method on the separately-locked `FocusTracker`.
fn finalize_phase2_process(
    capture: FinalizeCapture,
    is_current: &dyn Fn() -> bool,
) -> Result<FinalizeCommit, String> {
    let FinalizeCapture {
        session_id,
        stop_started_at,
        mut transcript,
        send_enter,
        mut ai_transform,
        audio_secs,
        focus,
        last_delivery_text,
        local_only_stop,
    } = capture;

    // Standalone voice commands are a Pro feature (root CLAUDE.md §6). The
    // SAME classifier the worker's streaming auto-fire uses decides command vs
    // dictation here — one source of truth (catalog command, app launch, or
    // macOS shortcut), so the two can't drift. KWS owns zephyr stop/send/cancel;
    // matching final-ASR phrases remain dictation here.
    let mut classified_action =
        crate::command_classify::classify_action(&transcript, crate::settings::is_pro());
    // "summarize" is catalog-listed so the worker's streaming probe AUTO-FIRES
    // on the bare word like any other standalone command — but its action is
    // not a keystroke. Reroute the semantic token into the ai-transform
    // selection lanes below (captured UIA/AX selection, else the Ctrl+C
    // copy-fallback) by leaving the transcript as the bare trigger word.
    // The catalog maps a "__"-prefixed token to CommandAction::Special with
    // the prefix stripped (token_to_action) — match THAT, not a KeySequence
    // chord (field bug 2026-07-16: the KeySequence match never fired and the
    // raw dispatch errored "unknown special op", sticking the pill on error).
    if matches!(
        &classified_action,
        Some(heardright_core::command::CommandAction::Special { op })
            if op == "summarize_selection"
    ) {
        tracing::info!("standalone_summarize_command_rerouted_to_selection_lane");
        classified_action = None;
        transcript = "summarize".to_string();
        ai_transform = None;
    }
    if let Some(action) = classified_action {
        let last_text = last_delivery_text.as_deref();
        // Windows/Linux: command chords + keystrokes go via SendInput to the
        // FOREGROUND window. Restore the captured target first so "undo"/"copy"/
        // "switch window" land in the user's app, not the pill or whatever drifted
        // into focus — exactly what the dictation path below does before pasting.
        // (macOS dispatches via CGEvent and already works without an explicit
        // restore, so leave that path untouched.)
        #[cfg(not(target_os = "macos"))]
        {
            let target = {
                let f = focus.lock();
                f.captured_target().or_else(|| f.current_target())
            };
            if let Some(target) = target {
                let _ = restore_and_verify(&target);
            }
        }
        // A Cancel or a new Start that raced ahead of classification/focus-
        // restore above must stop this command short of actually dispatching
        // — the next step is the OS-visible one.
        if discard_if_superseded(is_current, &session_id, "before_standalone_dispatch") {
            return Ok(FinalizeCommit::discard());
        }
        // A recognized command that fails to dispatch (no macOS equivalent,
        // unknown key, missing permission) must STILL return the engine to idle.
        // Propagating the error with `?` skipped the state reset, so no terminal
        // event fired and the pill hung in Processing forever (the "caps lock
        // kept processing" bug). Recover to armed either way — but carry the
        // failure so the shell can show it instead of faking success.
        //
        // macOS CGEvent dispatch silently no-ops without the Accessibility
        // grant — pre-check so "undo" without permissions is a visible
        // failure, not silence indistinguishable from success.
        #[cfg(target_os = "macos")]
        if !crate::macos_input::accessibility_trusted(false) {
            tracing::warn!("command dispatch blocked: accessibility not granted");
            trace_stop_to_engine_outcome(stop_started_at, "standalone_command_no_accessibility");
            return Ok(FinalizeCommit::outcome_only(
                FinalizeOutcome::CommandFailed {
                    message: "accessibility_not_granted".to_string(),
                },
            ));
        }
        match crate::command_dispatch::dispatch_with_last_text(&action, last_text) {
            Ok(outcome) => {
                tracing::info!(
                    action = outcome.action,
                    detail = %outcome.detail,
                    "command dispatched"
                );
                trace_stop_to_engine_outcome(stop_started_at, "standalone_command");
                return Ok(FinalizeCommit::outcome_only(
                    FinalizeOutcome::CommandDispatched {
                        action: outcome.action.to_string(),
                        detail: outcome.detail,
                    },
                ));
            }
            Err(err) => {
                tracing::warn!(
                    code = err.code,
                    message = %err.message,
                    "command dispatch failed; recovering to armed"
                );
                trace_stop_to_engine_outcome(stop_started_at, "standalone_command_failed");
                return Ok(FinalizeCommit::outcome_only(
                    FinalizeOutcome::CommandFailed {
                        message: err.message,
                    },
                ));
            }
        }
    }
    let polish_target = {
        let f = focus.lock();
        f.captured_target().or_else(|| f.current_target())
    };
    // Focused-field AX/UIA context: captured at record start, REFRESHED here at
    // stop (the field/selection may have changed mid-dictation — capture-at-
    // start-and-stop is the pattern that grounds rewrite/edit flows). Gated
    // on AI polish being on; secure fields are never read. Falls back to the
    // start capture when the fresh read returns nothing.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let captured_field = {
        let fresh = if crate::l3_cleanup::cleanup_enabled() {
            crate::focus::focused_field_context(crate::focus::FIELD_CONTEXT_MAX_CHARS)
        } else {
            None
        };
        fresh.or_else(|| focus.lock().captured_field())
    };
    // A bare "prompt" / "summarize" is a transform command only when the
    // user has explicitly selected non-empty text. Resolve it after the
    // fresh stop-time UIA/AX capture so the selection—not the command word—
    // becomes the L2/L3 source and the active selection receives the paste.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let selected_transform_source = if ai_transform.is_none() {
        captured_field
            .as_ref()
            .and_then(|field| {
                heardright_core::text_pipeline::parse_selected_text_ai_transform_command(
                    &transcript,
                    field.selected_text.as_deref(),
                )
            })
            .map(|command| {
                tracing::info!(intent = ?command.intent, "ai_transform_selection_detected");
                transcript = command.clean_text;
                ai_transform = Some(command.intent);
            })
            .is_some()
    } else {
        false
    };
    // Copy-fallback (Adrian, 2026-07-16): a selection in a READ-ONLY region
    // (chat transcript, rendered web page) is invisible to the focused-control
    // UIA read — focused_text_input=false, selected_text=None — so a bare
    // "summarize" previously died as ordinary dictation. When the utterance IS
    // the bare trigger and no selection was captured, fetch the selection with
    // a clipboard-preserving synthetic Ctrl+C. Runs in the UNLOCKED phase 2,
    // so the fetch's ~300ms worst-case poll never holds the runtime mutex.
    #[cfg(target_os = "windows")]
    let selected_transform_source = selected_transform_source
        || (ai_transform.is_none()
            && heardright_core::text_pipeline::is_bare_summarize_trigger(&transcript)
            && match crate::delivery::fetch_selected_text_via_copy() {
                Some(selection) => {
                    tracing::info!("ai_transform_selection_via_copy_fallback");
                    transcript = selection;
                    ai_transform =
                        Some(heardright_core::text_pipeline::AiTransformIntent::Summarize);
                    true
                }
                None => {
                    tracing::info!("ai_transform_copy_fallback_found_no_selection");
                    false
                }
            });
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let context_field_text = if selected_transform_source {
        captured_field.as_ref().and_then(|field| {
            crate::focus::field_text_without_selection(
                field.field_text.as_deref(),
                field.selected_text.as_deref(),
            )
        })
    } else {
        captured_field
            .as_ref()
            .and_then(|field| field.field_text.clone())
    };
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let context_selected_text = if selected_transform_source {
        None
    } else {
        captured_field
            .as_ref()
            .and_then(|field| field.selected_text.clone())
    };
    // Country/region context is auto-detected from the OS locale (e.g.
    // "en-IN"), never typed by the user — sent to the polish LLM for spelling
    // and tone only. Replaces the removed manual "Writing region" field.
    let writing_region = sys_locale::get_locale();
    let polish_context = crate::text_polish::DictationPolishContext {
        audio_secs,
        app_name: polish_target
            .as_ref()
            .and_then(|target| target.process_name.as_deref()),
        window_title: polish_target
            .as_ref()
            .and_then(|target| target.window_title.as_deref()),
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        field_text: context_field_text.as_deref(),
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        selected_text: context_selected_text.as_deref(),
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        field_context_available: captured_field.is_some(),
        writing_region: writing_region.as_deref(),
        ..Default::default()
    };
    // raw = the deterministic LOCAL-ONLY polish (no AI) — what "undo AI" shows.
    // polished = the full result (normal path: local + L1 app-aware AI inside
    // polish_dictation; ai_transform path: local-only here, L2/L3 applied
    // in the match below). Capturing local_only here is the ONLY correct raw point —
    // polish_dictation runs L3 internally, so reading `transcript`
    // afterward would already be AI-polished.
    // Continuation casing: when the focused field already ends mid-sentence,
    // the dictation continues it — keep the first word lowercase. Applies to
    // the normal insertion lane only (transform output replaces a selection,
    // which is standalone text).
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let capitalize_start = ai_transform.is_some()
        || !crate::text_polish::continues_mid_sentence(context_field_text.as_deref());
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let capitalize_start = true;
    let local_only = crate::text_polish::polish_local_only_with(&transcript, capitalize_start);
    let snippets = crate::settings::snippets();
    let expand = |t: &str| {
        if snippets.is_empty() {
            t.to_string()
        } else {
            heardright_core::text_pipeline::expand_snippets(t, &snippets)
        }
    };
    let raw_candidate = expand(&local_only);
    let ai_context = crate::l3_cleanup::PolishContext {
        app_name: polish_target
            .as_ref()
            .and_then(|target| target.process_name.clone()),
        window_title: polish_target
            .as_ref()
            .and_then(|target| target.window_title.clone()),
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        field_text: context_field_text.clone(),
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        selected_text: context_selected_text.clone(),
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        field_context_available: captured_field.is_some(),
        vocabulary: crate::vocabulary::terms(),
        writing_region: writing_region.clone(),
        sound_alikes: crate::vocabulary::sound_alike_pairs(),
        ..Default::default()
    };
    // Explicit dogfood evaluation mode: capture one production-real input,
    // fan it out to the frozen prompt/model matrix, and never type it into
    // the target app. This stays outside normal delivery and is inert unless
    // the operator sets HEARDRIGHT_AI_EVAL_MODE.
    if crate::l3_cleanup::live_eval_suppresses_delivery() {
        let (lane, eval_input) = match ai_transform {
            Some(heardright_core::text_pipeline::AiTransformIntent::Prompt) => {
                (crate::l3_cleanup::LiveEvalLane::L2, raw_candidate.as_str())
            }
            Some(heardright_core::text_pipeline::AiTransformIntent::Summarize) => {
                (crate::l3_cleanup::LiveEvalLane::L3, raw_candidate.as_str())
            }
            None => (crate::l3_cleanup::LiveEvalLane::L1, local_only.as_str()),
        };
        match crate::l3_cleanup::run_live_evaluation(eval_input, &ai_context, lane) {
            Ok(path) => tracing::info!(output = %path.display(), "live_eval_case_complete"),
            Err(error) => tracing::error!(%error, "live_eval_case_failed"),
        }
        trace_stop_to_engine_outcome(stop_started_at, "live_eval_no_delivery");
        return Ok(FinalizeCommit::outcome_only(FinalizeOutcome::ResetToArmed));
    }

    // Cancel/new-Start check before the (potentially multi-second) cloud L2/L3
    // polish call — no point spending an HTTP round-trip on a session that's
    // already been superseded.
    if discard_if_superseded(is_current, &session_id, "before_polish") {
        return Ok(FinalizeCommit::discard());
    }

    // Cancel-to-history (`local_only_stop`) skips BOTH cloud lanes exactly
    // like a disabled/no-consent stop would, but unconditionally — the
    // deterministic local (L0) polish above stands as the final transcript,
    // and `polish_dictation` (L1 app-aware cloud polish) and the ai-transform
    // L2/L3 lanes below are never called, regardless of settings/consent or
    // whether the utterance carried a "prompt"/"summarize" tail. A cancelled
    // utterance's text must never leave the device.
    let polished = if local_only_stop || ai_transform.is_some() {
        local_only.clone()
    } else {
        crate::text_polish::polish_dictation(&transcript, polish_context)
    };
    let transcript = expand(&polished);
    let transcript = if local_only_stop {
        transcript
    } else {
        match ai_transform {
            Some(heardright_core::text_pipeline::AiTransformIntent::Prompt) => {
                match crate::l3_cleanup::prompt_polish_outcome(&transcript, &ai_context) {
                    crate::l3_cleanup::CleanupOutcome::Cleaned(prompt) => prompt,
                    crate::l3_cleanup::CleanupOutcome::Failed { error_class, .. } => {
                        crate::l3_cleanup::record_local_fallback();
                        tracing::warn!(error_class, "l3_prompt_fallback_to_cleaned_transcript");
                        transcript
                    }
                    crate::l3_cleanup::CleanupOutcome::Skipped { reason, .. } => {
                        tracing::warn!(reason, "l3_prompt_skipped_using_cleaned_transcript");
                        transcript
                    }
                }
            }
            Some(heardright_core::text_pipeline::AiTransformIntent::Summarize) => {
                match crate::l3_cleanup::summarize_outcome(&transcript, &ai_context) {
                    crate::l3_cleanup::CleanupOutcome::Cleaned(summary) => summary,
                    crate::l3_cleanup::CleanupOutcome::Failed { error_class, .. } => {
                        crate::l3_cleanup::record_local_fallback();
                        tracing::warn!(error_class, "l3_summary_fallback_to_cleaned_transcript");
                        transcript
                    }
                    crate::l3_cleanup::CleanupOutcome::Skipped { reason, .. } => {
                        tracing::warn!(reason, "l3_summary_skipped_using_cleaned_transcript");
                        transcript
                    }
                }
            }
            None => transcript,
        }
    };
    owner_diagnostics::record_event(serde_json::json!({
        "event": "delivery_transcript",
        "session_id": &session_id,
        "raw_transcript": &raw_candidate,
        "delivered_transcript": &transcript,
        "send_enter": send_enter,
    }));
    if transcript.trim().is_empty() {
        let record = DeliveryRecord::new(
            delivery_id_for(""),
            "",
            DeliveryOutcome::CopiedFallback {
                reason: CopyFallbackReason::EmptyTranscript,
            },
            snapshot_target(),
        );
        trace_stop_to_engine_outcome(stop_started_at, "empty_transcript");
        return Ok(FinalizeCommit::with_record(
            FinalizeOutcome::Delivery {
                record: record.clone(),
                send_enter,
            },
            record,
            false,
        ));
    }
    let raw_text = if raw_candidate != transcript {
        Some(raw_candidate)
    } else {
        None
    };
    let recording_ms = audio_secs.map(|s| (s.max(0.0) * 1000.0) as u64);
    // Cancel/new-Start check before handing off to the shell — the last
    // engine-side point where stopping short avoids any OS-visible action.
    // Phase 3's generation check is the authoritative guard; this is belt-
    // and-suspenders so we don't emit a sidecar event for a dead session.
    if discard_if_superseded(is_current, &session_id, "before_shell_handoff") {
        return Ok(FinalizeCommit::discard());
    }
    // Selected-text safety (locked 2026-07-14): standalone "prompt" /
    // "summarize" only makes sense when the user had selected text and
    // that selection is still present at delivery. Re-read the focused
    // field right before handoff; if the live selection is empty, fall
    // back to copy instead of letting the shell type the transformed
    // result into a field the user changed.
    //
    // `!local_only_stop`: `copy_fallback_record` performs a REAL OS
    // clipboard write and returns `FinalizeOutcome::Delivery`, which the
    // shell routes to `accept_sidecar_delivery` — a path that does not
    // know about `cancel_to_history_session` at all. A cancel-to-history
    // finalize must never touch the clipboard or bypass the shell's
    // cancel-aware routing, so this safety-net branch is skipped entirely
    // for that stop; the plain `Transcript` outcome below already carries
    // the local-only text with no clipboard/paste side effect.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    if ai_transform.is_some() && !local_only_stop {
        let live_selection =
            crate::focus::focused_field_context(crate::focus::FIELD_CONTEXT_MAX_CHARS)
                .and_then(|ctx| ctx.selected_text)
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty());
        if live_selection.is_none() {
            tracing::warn!(
                intent = ?ai_transform,
                "selected_text_stale_falling_back_to_copy"
            );
            let target = focus
                .lock()
                .captured_target()
                .unwrap_or_else(snapshot_target);
            let record =
                copy_fallback_record(&transcript, CopyFallbackReason::FocusChanged, target)
                    .with_raw(raw_text, recording_ms);
            trace_stop_to_engine_outcome(stop_started_at, "stale_selection_copy_fallback");
            return Ok(FinalizeCommit::with_record(
                FinalizeOutcome::Delivery {
                    record: record.clone(),
                    send_enter,
                },
                record,
                false,
            ));
        }
    }
    // Both platforms hand the finished transcript to the shell for delivery
    // (clipboard + paste + optional Enter). The shell owns focus validation,
    // paste-settle timing, session generation, pill state, and history —
    // the engine's job ends at producing the final text.
    trace_stop_to_engine_outcome(stop_started_at, "shell_delivery");
    Ok(FinalizeCommit::outcome_only(FinalizeOutcome::Transcript {
        text: transcript,
        send_enter,
        raw_text,
        recording_ms,
    }))
}
