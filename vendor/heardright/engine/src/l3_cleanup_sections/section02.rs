pub fn prompt_polish_outcome(input: &str, context: &PolishContext) -> CleanupOutcome {
    // Same safety boundary as app_polish_outcome: scrub captured context
    // before it reaches any provider payload.
    let context = &sanitize_context(context);
    transform_outcome(
        input,
        PROMPT_POLISH_VERSION,
        "l2_prompt_polish_no_provider_available",
        "l2_prompt_polish_all_providers_failed",
        |spec, input| build_prompt_payload(spec, input, context),
        |input, output| {
            normalize_output_with_extra(input, output, 1_200)
                .map(|out| scrub_dashes(&out))
                .filter(|out| digits_preserved(input, out))
        },
        |_, _| {},
    )
}

fn transform_outcome<F, N, S>(
    input: &str,
    prompt_version: &'static str,
    no_provider_log: &'static str,
    failed_log: &'static str,
    build_payload: F,
    normalize: N,
    on_success: S,
) -> CleanupOutcome
where
    F: Fn(&ProviderSpec, &str) -> Value,
    N: Fn(&str, &str) -> Option<String>,
    S: Fn(&str, &str),
{
    let input = match preflight_input(input) {
        Ok(input) => input,
        Err(outcome) => return outcome,
    };

    ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    let providers = provider_specs(prompt_version);
    if providers.is_empty() {
        FAILURES.fetch_add(1, Ordering::Relaxed);
        tracing::warn!(event = no_provider_log, "ai_polish_no_provider_available");
        emit_runtime_diagnostic("error", "polish", no_provider_log, true);
        return CleanupOutcome::Failed {
            error_class: "no_provider",
            circuit_open: false,
        };
    }

    let total_timeout = total_timeout_for_input(input);
    let start = Instant::now();
    let deadline = start + total_timeout;
    let mut last_error = "not_attempted";
    let mut any_provider_attempted = false;

    for (fallback_index, spec) in providers.iter().enumerate() {
        let now = Instant::now();
        // A fallback attempt with almost no budget left is guaranteed dead
        // (field-verified 2026-07-16: the second provider got 94ms of a fixed
        // 900ms budget and instantly timed out). Don't fire doomed calls —
        // fall back to the local polish instead. The FIRST fallback is exempt:
        // it gets a real floor below instead of a doomed sliver.
        const MIN_ATTEMPT_BUDGET: Duration = Duration::from_millis(300);
        if fallback_index >= 2 && now + MIN_ATTEMPT_BUDGET >= deadline {
            last_error = "timeout";
            break;
        }
        if provider_circuit_open(spec.provider) {
            record_skip();
            trace_skip("provider_circuit_open", "open");
            last_error = "circuit_open";
            continue;
        }
        any_provider_attempted = true;
        let remaining = deadline
            .saturating_duration_since(now)
            .min(provider_timeout_for_budget(total_timeout));
        // Field-verified 2026-07-19: after a primary timeout the first
        // fallback inherited only the ~1/3 budget remainder (~450ms) — not
        // enough for a cold TLS handshake plus inference; across the whole
        // payload log it never once succeeded. Give it a real floor (worst
        // case stretches one dictation by <1s); later attempts still respect
        // the hard deadline via the break above.
        let remaining = if fallback_index == 1 {
            remaining.max(provider_timeout_for_budget(total_timeout))
        } else {
            remaining
        };
        trace_attempt(spec, total_timeout, fallback_index, prompt_version);

        let attempt_start = Instant::now();
        match call_provider(spec, input, remaining, &build_payload, prompt_version) {
            Ok(output) => {
                if let Some(output) = normalize(input, &output) {
                    on_success(input, &output);
                    record_success(spec.provider);
                    trace_result(
                        spec,
                        "success",
                        attempt_start.elapsed(),
                        "",
                        "closed",
                        prompt_version,
                    );
                    return CleanupOutcome::Cleaned(output);
                }
                last_error = "rejected_output";
                record_failure(spec.provider);
                trace_result(
                    spec,
                    "failure",
                    attempt_start.elapsed(),
                    last_error,
                    "closed",
                    prompt_version,
                );
            }
            Err(error_class) => {
                last_error = error_class;
                record_failure(spec.provider);
                trace_result(
                    spec,
                    "failure",
                    attempt_start.elapsed(),
                    error_class,
                    "closed",
                    prompt_version,
                );
            }
        }
    }

    if !any_provider_attempted && last_error == "circuit_open" {
        return CleanupOutcome::Skipped {
            reason: "circuit_open",
            circuit_open: true,
        };
    }
    let health = health();
    tracing::warn!(
        error_class = last_error,
        circuit_open = health.circuit_open,
        consecutive_failures = health.consecutive_failures,
        prompt_version,
        event = failed_log,
        "ai_polish_all_providers_failed"
    );
    CleanupOutcome::Failed {
        error_class: last_error,
        circuit_open: health.circuit_open,
    }
}

pub fn summarize_outcome(input: &str, context: &PolishContext) -> CleanupOutcome {
    let context = &sanitize_context(context);
    transform_outcome(
        input,
        SUMMARY_PROMPT_VERSION,
        "l3_summary_no_provider_available",
        "l3_summary_all_providers_failed",
        |spec, input| build_summary_payload(spec, input, context),
        // Summary legitimately condenses, so no digit-preservation guard —
        // but the dash ban applies to every lane.
        |input, output| normalize_output(input, output).map(|out| scrub_dashes(&out)),
        |_, _| {},
    )
}

/// Whether AI polish is enabled at all — the privacy gate for capturing any
/// machine-read context (focused-field text) in the first place.
pub fn cleanup_enabled() -> bool {
    env_true("HEARDRIGHT_L3_CLEANUP")
}

fn preflight_input(input: &str) -> Result<&str, CleanupOutcome> {
    let input = input.trim();
    if input.is_empty() {
        record_skip();
        trace_skip("empty_input", "closed");
        return Err(CleanupOutcome::Skipped {
            reason: "empty_input",
            circuit_open: false,
        });
    }
    if !env_true("HEARDRIGHT_L3_CLEANUP") {
        record_skip();
        trace_skip("disabled", "closed");
        return Err(CleanupOutcome::Skipped {
            reason: "disabled",
            circuit_open: false,
        });
    }
    if !env_true("HEARDRIGHT_L3_CLOUD_CONSENT") {
        record_skip();
        trace_skip("missing_consent", "closed");
        return Err(CleanupOutcome::Skipped {
            reason: "missing_consent",
            circuit_open: false,
        });
    }
    if input.chars().count() > max_input_chars() {
        record_skip();
        trace_skip("input_too_large", "closed");
        return Err(CleanupOutcome::Skipped {
            reason: "input_too_large",
            circuit_open: false,
        });
    }
    Ok(input)
}

#[cfg(test)]
fn cached_cleanup(input: &str) -> Option<String> {
    CLEANUP_CACHE
        .get()
        .and_then(|cache| {
            cache
                .lock()
                .iter()
                .find(|entry| entry.input == input)
                .cloned()
        })
        .map(|entry| entry.cleaned)
}

#[cfg(test)]
pub fn cached_prefix(input: &str) -> Option<(usize, String)> {
    CLEANUP_CACHE
        .get()
        .and_then(|cache| {
            cache
                .lock()
                .iter()
                .filter(|entry| entry.input.len() < input.len())
                .filter(|entry| input.starts_with(&entry.input))
                .filter(|entry| {
                    input[entry.input.len()..]
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_whitespace() || ch.is_ascii_punctuation())
                })
                .max_by_key(|entry| entry.input.len())
                .cloned()
        })
        .map(|entry| (entry.input.len(), entry.cleaned))
}

#[cfg(test)]
fn store_cleanup(input: &str, cleaned: &str) {
    let cache = CLEANUP_CACHE.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = cache.lock();
    if let Some(entry) = guard.iter_mut().find(|entry| entry.input == input) {
        entry.cleaned = cleaned.to_string();
        return;
    }
    guard.push(CachedCleanup {
        input: input.to_string(),
        cleaned: cleaned.to_string(),
    });
    if guard.len() > CLEANUP_CACHE_MAX {
        let overflow = guard.len() - CLEANUP_CACHE_MAX;
        guard.drain(0..overflow);
    }
}
