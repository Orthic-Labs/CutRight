#[cfg(test)]
mod tests {
    use super::*;
    // The conditional ladder (locked 2026-07-14) reads the BYOK provider keys
    // straight off the process env, no longer gated behind HEARDRIGHT_L3_ROSTER.
    // A developer shell that exports GROQ_API_KEY (the bakeoff machines all do)
    // would otherwise leak a real Groq route into every provider_specs test, so
    // clear them by default and let a test opt back in by naming them in `vars`.
    const BYOK_PROVIDER_KEYS: &[&str] = &[
        "GROQ_API_KEY",
        "CEREBRAS_API_KEY",
        "OPENROUTER_API_KEY",
        "NVIDIA_API_KEY",
        "NIM_API_KEY",
    ];

    fn with_env(vars: &[(&str, Option<&str>)], test: impl FnOnce()) {
        // ONE crate-wide env lock (crate::test_support, shared with
        // apple_foundation::tests on macOS) — two separate locks let modules
        // race on the same HEARDRIGHT_* vars.
        let _guard = crate::test_support::test_env_guard();
        let previous: Vec<(&str, Option<String>)> = BYOK_PROVIDER_KEYS
            .iter()
            .copied()
            .chain(vars.iter().map(|(key, _)| *key))
            .map(|key| (key, std::env::var(key).ok()))
            .collect();
        reset_circuit_for_tests();
        for key in BYOK_PROVIDER_KEYS {
            std::env::remove_var(key);
        }
        for (key, value) in vars {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
        test();
        for (key, value) in previous {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
        reset_circuit_for_tests();
    }

    fn reset_circuit_for_tests() {
        if let Some(circuit) = CIRCUIT.get() {
            *lock_circuit(circuit) = Circuit::default();
        }
        if let Some(agents) = AGENTS.get() {
            agents.lock().clear();
        }
        if let Some(cache) = CLEANUP_CACHE.get() {
            cache.lock().clear();
        }
        #[cfg(target_os = "macos")]
        APPLE_CONSECUTIVE_FAILURES.store(0, Ordering::Relaxed);
        ATTEMPTS.store(0, Ordering::Relaxed);
        SUCCESSES.store(0, Ordering::Relaxed);
        FAILURES.store(0, Ordering::Relaxed);
        SKIPS.store(0, Ordering::Relaxed);
        LOCAL_FALLBACKS.store(0, Ordering::Relaxed);
        CIRCUIT_OPENS.store(0, Ordering::Relaxed);
        LIVE_EVAL_SEQUENCE.store(0, Ordering::Relaxed);
    }

    #[test]
    fn l3_agents_are_reused_by_timeout_bucket() {
        with_env(&[], || {
            let first = agent_for_timeout(Duration::from_millis(949));
            let second = agent_for_timeout(Duration::from_millis(900));
            let other = agent_for_timeout(Duration::from_millis(1_000));
            assert!(Arc::ptr_eq(&first, &second));
            assert!(!Arc::ptr_eq(&first, &other));
        });
    }

    #[test]
    fn l3_agent_timeout_bucket_never_exceeds_deadline() {
        assert_eq!(timeout_bucket_ms(Duration::from_millis(949)), 900);
        assert_eq!(timeout_bucket_ms(Duration::from_millis(250)), 200);
        assert_eq!(timeout_bucket_ms(Duration::from_millis(1)), 1);
    }

    #[test]
    fn l3_default_final_budget_has_provider_jitter_room() {
        with_env(
            &[
                ("HEARDRIGHT_L3_TOTAL_TIMEOUT_MS", None),
                ("HEARDRIGHT_L3_PROVIDER_TIMEOUT_MS", None),
            ],
            || {
                assert_eq!(total_timeout(), Duration::from_millis(1_500));
                assert_eq!(provider_timeout(), Duration::from_millis(1_500));
            },
        );
    }

    #[test]
    fn openrouter_requires_exact_validation_marker() {
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", Some("openrouter")),
                ("OPENROUTER_API_KEY", Some("test-key")),
                (
                    "HEARDRIGHT_L3_OPENROUTER_MODEL",
                    Some("deepseek/deepseek-v4-flash"),
                ),
                (
                    "HEARDRIGHT_L3_OPENROUTER_VALIDATED_FOR",
                    Some("different|l3c_v1"),
                ),
            ],
            || assert!(provider_specs(PROMPT_VERSION).is_empty()),
        );

        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", Some("openrouter")),
                ("OPENROUTER_API_KEY", Some("test-key")),
                (
                    "HEARDRIGHT_L3_OPENROUTER_MODEL",
                    Some("deepseek/deepseek-v4-flash"),
                ),
                (
                    "HEARDRIGHT_L3_OPENROUTER_VALIDATED_FOR",
                    Some("deepseek/deepseek-v4-flash|l3c_v1"),
                ),
            ],
            || assert_eq!(provider_specs(PROMPT_VERSION).len(), 1),
        );

        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", Some("openrouter")),
                ("OPENROUTER_API_KEY", Some("test-key")),
                (
                    "HEARDRIGHT_L3_OPENROUTER_MODEL",
                    Some("google/gemini-2.5-flash"),
                ),
                ("HEARDRIGHT_L3_OPENROUTER_VALIDATED_FOR", None),
            ],
            || assert!(provider_specs(APP_PROMPT_VERSION).is_empty()),
        );
    }

    #[test]
    fn new_prompt_versions_do_not_inherit_old_openrouter_validation() {
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", Some("openrouter")),
                ("OPENROUTER_API_KEY", Some("test-key")),
                (
                    "HEARDRIGHT_L3_OPENROUTER_MODEL",
                    Some(L2_OPENROUTER_FALLBACK),
                ),
                (
                    "HEARDRIGHT_L3_OPENROUTER_VALIDATED_FOR",
                    Some("qwen/qwen3-32b|l2_prompt_polish_prompt_v2"),
                ),
            ],
            || assert!(provider_specs(PROMPT_POLISH_VERSION).is_empty()),
        );
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", Some("openrouter")),
                ("OPENROUTER_API_KEY", Some("test-key")),
                ("HEARDRIGHT_L3_OPENROUTER_MODEL", Some(L3_GROQ_PRIMARY)),
                (
                    "HEARDRIGHT_L3_OPENROUTER_VALIDATED_FOR",
                    Some("qwen/qwen3.6-27b|l3_summarize_prompt_v1"),
                ),
            ],
            || assert!(provider_specs(SUMMARY_PROMPT_VERSION).is_empty()),
        );
    }

    #[test]
    fn payload_uses_strict_prompt_and_versioned_shape() {
        let payload = build_payload(
            &ProviderSpec {
                provider: Provider::Groq,
                base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: "test-key".to_string(),
                model: "llama-3.3-70b-versatile".to_string(),
            },
            "send the in voice today",
        );
        assert_eq!(payload["temperature"], 0);
        assert_eq!(payload["stream"], false);
        assert_eq!(payload["messages"][0]["content"], BASE_SYS);
        assert!(payload["messages"][1]["content"]
            .as_str()
            .unwrap()
            .contains("Cleaned transcript:"));
        assert_eq!(PROMPT_VERSION, "l3c_v1");
    }

    #[test]
    fn summary_payload_uses_v3_prompt_context_and_chat_shape() {
        let payload = build_summary_payload(
            &ProviderSpec {
                provider: Provider::Groq,
                base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: "test-key".to_string(),
                model: "llama-3.3-70b-versatile".to_string(),
            },
            "the launch call covered pricing blockers and model hosting",
            &PolishContext {
                app_name: Some("Slack".to_string()),
                window_title: Some("Launch planning".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(payload["temperature"], 0);
        assert_eq!(payload["stream"], false);
        let system = payload["messages"][0]["content"].as_str().unwrap();
        let user = payload["messages"][1]["content"].as_str().unwrap();
        assert!(system.starts_with(SUMMARY_SYS_CORE));
        assert!(system.contains("Use 1-3 concise prose sentences"));
        assert!(user.contains("Active app: Slack"));
        assert!(user.contains("Active window/title: Launch planning"));
        assert!(user.contains("Environment: today is "));
        assert!(user.contains("Summary:"));
        assert_eq!(SUMMARY_PROMPT_VERSION, "l3_summarize_prompt_v3");
    }

    #[test]
    fn app_payload_uses_app_prompt_and_target_context() {
        let payload = build_app_payload(
            &ProviderSpec {
                provider: Provider::Groq,
                base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: "test-key".to_string(),
                model: "llama-3.1-8b-instant".to_string(),
            },
            "the footer has too much white space and the hyperlinks are messy",
            &PolishContext {
                app_name: Some("Slack".to_string()),
                window_title: Some("LaundryDaddy launch thread".to_string()),
                field_text: Some("Shipping tonight. Last blocker is the footer.".to_string()),
                ..Default::default()
            },
        );
        let user = payload["messages"][1]["content"].as_str().unwrap();
        let system = payload["messages"][0]["content"].as_str().unwrap();
        assert_eq!(payload["temperature"], 0);
        assert_eq!(payload["stream"], false);
        // Slack resolves to the chat block on top of the shared core.
        assert!(system.starts_with(APP_SYS_CORE));
        assert!(system.contains("chat/messaging app"));
        assert!(user.contains("Active app: Slack"));
        assert!(user.contains("App kind: chat/messaging app"));
        assert!(user.contains("Active window/title: LaundryDaddy launch thread"));
        assert!(user.contains("Existing text in the target field"));
        assert!(user.contains("Last blocker is the footer."));
        assert!(!user.contains("Environment: today is "));
        assert!(user.contains("Polished output:"));
        assert_eq!(APP_PROMPT_VERSION, "l1_app_polish_prompt_v7");
    }

    #[test]
    fn app_payload_without_context_omits_unknown_lines_and_uses_core_prompt() {
        let payload = build_app_payload(
            &ProviderSpec {
                provider: Provider::Groq,
                base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: "test-key".to_string(),
                model: "llama-3.1-8b-instant".to_string(),
            },
            "send the invoice tomorrow morning",
            &PolishContext::default(),
        );
        let user = payload["messages"][1]["content"].as_str().unwrap();
        assert_eq!(payload["messages"][0]["content"], APP_SYS_CORE);
        assert!(user.contains("No active-app context available."));
        assert!(!user.contains("unknown app"));
        assert!(!user.contains("unknown window"));
    }

    #[test]
    fn app_payload_excludes_client_names_unless_user_added() {
        let spec = ProviderSpec {
            provider: Provider::Groq,
            base_url: "https://api.groq.com/openai/v1".to_string(),
            api_key: "test-key".to_string(),
            model: "llama-3.1-8b-instant".to_string(),
        };

        let default_payload = build_app_payload(
            &spec,
            "so that the view right agent can implement it there as well",
            &PolishContext::default(),
        );
        let default_prompt = default_payload["messages"][1]["content"].as_str().unwrap();
        let default_system = default_payload["messages"][0]["content"].as_str().unwrap();
        assert!(default_prompt.contains("the view right agent"));
        assert!(!default_prompt.contains("HeardRight"));
        assert!(!default_system.contains("HeardRight"));
        assert!(!default_prompt.contains("OneClickDrive"));

        let explicit_payload = build_app_payload(
            &spec,
            "update the client site",
            &PolishContext {
                vocabulary: vec!["OneClickDrive".to_string()],
                ..Default::default()
            },
        );
        let explicit_prompt = explicit_payload["messages"][1]["content"].as_str().unwrap();
        assert!(explicit_prompt.contains("OneClickDrive"));
    }

    #[test]
    fn prompt_payload_uses_v5_prompt_and_code_shape() {
        let payload = build_prompt_payload(
            &ProviderSpec {
                provider: Provider::Groq,
                base_url: "https://api.groq.com/openai/v1".to_string(),
                api_key: "test-key".to_string(),
                model: "llama-3.1-8b-instant".to_string(),
            },
            "make the pricing page clearer and mention the two plans",
            &PolishContext {
                app_name: Some("Cursor".to_string()),
                window_title: Some("pricing-page.tsx".to_string()),
                ..Default::default()
            },
        );
        let user = payload["messages"][1]["content"].as_str().unwrap();
        assert_eq!(payload["temperature"], 0);
        assert_eq!(payload["stream"], false);
        let system = payload["messages"][0]["content"].as_str().unwrap();
        assert!(system.starts_with(PROMPT_SYS_CORE));
        assert!(system.contains("The target is a coding agent or code editor."));
        assert!(system.contains("Do not invent requirements, acceptance criteria"));
        assert!(system.contains("Use the transcript's language"));
        assert!(user.contains("Active app: Cursor"));
        assert!(user.contains("Active window/title: pricing-page.tsx"));
        assert!(user.contains("Environment: today is "));
        assert!(user.contains("Prompt:"));
        assert_eq!(PROMPT_POLISH_VERSION, "l2_prompt_polish_prompt_v6");
    }

    #[test]
    fn environment_line_has_frozen_machine_readable_shape() {
        assert_eq!(
            format_environment_line(
                "Tuesday, 2026-07-14",
                "Asia/Kolkata",
                "+05:30",
                "en-US"
            ),
            "Environment: today is Tuesday, 2026-07-14; timezone Asia/Kolkata (UTC+05:30); locale en-US."
        );
    }

    #[test]
    fn production_prompts_match_the_frozen_convergence_winners() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../bakeoff/prompt-convergence-v1/frozen-prompts.json");
        let frozen: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read frozen prompts"))
                .expect("parse frozen prompts");
        // Post-bakeoff amendment (Adrian, 2026-07-15): a hard no-em-dash rule was
        // added to ALL THREE core prompts (L1/L2/L3) because the models were
        // emitting em/en dashes, a well-known AI tell. frozen-prompts.json stays
        // UNTOUCHED (it is the evidence record of what the blind bakeoff scored);
        // production = frozen core + this recorded line, so stripping the line back
        // out must reproduce the frozen bytes exactly. Any OTHER drift still fails.
        const EM_DASH_RULE: &str = "\nNEVER use em dashes (—) or en dashes (–). They read as an AI tell and were not spoken. Use a comma, a period, parentheses, or a colon instead; use 'to' for numeric ranges.";
        // Post-bakeoff amendment (Adrian, 2026-07-16): dictating into a field
        // that already ends mid-sentence must not force-capitalize the first
        // word — the models were told to "ALWAYS FIX casing" with no continuation
        // rule, so they re-capitalized what the local continuation-aware pass
        // had correctly left lowercase. L1-only (insertion lane; L2/L3 replace a
        // selection with standalone text). frozen-prompts.json stays UNTOUCHED.
        const CONTINUATION_CASING_RULE: &str = "\nCONTINUATION CASING: when field_text is present and ends mid-sentence (no closing punctuation), the dictation continues that sentence at the insertion point: keep the first word lowercase unless it is a proper noun or the dictation clearly starts a new sentence. Never modify field_text itself.";
        const SPOKEN_STRUCTURE_RULE: &str = "\nSPOKEN STRUCTURE: when the speaker explicitly enumerates distinct points (for example \"1\", \"2\", \"3\", \"first\", \"second\", or \"three things\"), put each point on a separate line as a numbered list. Apply this in every app, including browser-hosted chat, email, and messaging. Never invent a list when numbers are ordinary values rather than point markers.";
        // Post-bakeoff amendment (Adrian, 2026-07-16): "the whole point of the
        // AI is to fix issues the ASR cannot, because the ASR is not context
        // aware" — payload-verified mishears ("by voice" -> "by boys",
        // "SellRight" -> "Cecil") survived polish because the frozen prompts
        // only allowed repairs "supported by runtime context or known terms".
        // Added to L1 and L2 cores; L3 summarizes and rewords anyway.
        const ASR_REPAIR_RULE: &str = "\nTRANSCRIPTION REPAIR: the transcript comes from speech recognition and can contain mishearings. When a word or phrase is nonsensical in its context and a phonetically similar alternative is clearly what was spoken, repair it (for example 'run your machine by boys' -> 'run your machine by voice'). Repair only when sound plus context make the intent unambiguous; never repair names, numbers, or technical identifiers this way.";
        // Post-audit amendments (Adrian, 2026-07-19), from the first payload-log
        // review of l1_app_polish_prompt_v4 on both machines (64 Mac + 83 Win
        // calls). Three measured failure classes: digit corruption ("1 5" ->
        // "1.5"), name-shaped garble smoothed into ordinary words ("heard ride"
        // -> "header", "Herdress" -> "her dress"), and approvals/hedges eaten as
        // filler ("Yeah, go ahead." deleted; "I think" dropped). NUMBER LITERALS
        // is belt to the deterministic digits_preserved guard's suspenders.
        // frozen-prompts.json stays UNTOUCHED as the bakeoff evidence record.
        const NUMBER_LITERALS_RULE: &str = "\nNUMBER LITERALS: never change, add, or drop a digit; never insert or remove a decimal point; never merge or split spaced digits into a different number. When unsure how digits were meant, keep them exactly as transcribed.";
        const NAME_GARBLE_RULE: &str = "\nIf a garbled span appears to be a name, product, or technical term and no known term or context resolves it, keep it verbatim; never smooth it into ordinary dictionary words.";
        const MEANING_WORDS_RULE: &str = "\nLeading acknowledgments and approvals ('yeah', 'yes', 'go ahead', 'sounds good') and hedges ('I think', 'maybe', 'probably') are meaning, never filler.";
        // Post-audit amendment (Adrian, 2026-07-27), from an owner-diagnostics
        // capture: "my transcriptions are redacted to" was delivered verbatim.
        // ASR_REPAIR_RULE could not catch it — it fires only when a phrase is
        // "nonsensical in its context", and "redacted to" parses fine. MINIMALITY
        // actively argued against it: "to" is a correct, correctly spelled word.
        // Exact homophones are the one class the acoustic model can NEVER decide
        // (to/too are the same phonemes, so no decoder or beam width helps) and
        // grammar alone resolves them — precisely the work the LLM lane exists
        // to do. frozen-prompts.json stays UNTOUCHED as the bakeoff evidence.
        const HOMOPHONE_GRAMMAR_RULE: &str = "\nHOMOPHONE GRAMMAR: exact homophones sound identical, so the speech recogniser cannot choose between them and grammar is the only thing that can. Fix the wrong member of a homophone pair whenever grammar makes the right one certain, EVEN THOUGH the transcribed word is itself a real, correctly spelled English word and the sentence still parses: to/too/two, there/their/they're, its/it's, your/you're, then/than, whose/who's, affect/effect, lose/loose, and 'of' for 'have' after a modal (could of -> could have). Example: 'my transcriptions are redacted to' -> 'my transcriptions are redacted too'. This is a stated exception to MINIMALITY: a homophone in the wrong grammatical role is not correct wording, it is a recognition artifact. Apply it only where grammar decides; when both readings are grammatical, keep what was transcribed.";
        // Post-audit amendment (Adrian, 2026-08-02): ‘oh no’ added to the
        // self-correction cue examples, shipped alongside the correction-aware
        // digit guard ("Let's meet at 7, oh no, 8" was mechanically vetoed by
        // strict digit-signature equality, so number self-corrections could
        // never ship). frozen-prompts.json stays UNTOUCHED as the bakeoff
        // evidence record; stripping the cue back out must reproduce it.
        const OH_NO_CUE_AMENDED: &str = "‘no, wait’, ‘oh no’, ‘I mean’";
        const OH_NO_CUE_FROZEN: &str = "‘no, wait’, ‘I mean’";
        // Post-review amendment (Adrian, 2026-08-02, v7): NUMBER LITERALS said
        // "never drop a digit" while SELF-CORRECTION PRECEDENCE requires
        // dropping the superseded value — a direct contradiction the model had
        // to resolve arbitrarily. The exception sentence makes the correction
        // carve-out explicit, matching the correction-aware digit guard.
        // frozen-prompts.json stays UNTOUCHED as the bakeoff evidence record.
        const NUMBER_LITERALS_CORRECTION_EXCEPTION: &str = " The one exception is an explicit self-correction: when the speaker replaces a number, keep only the final value and drop the superseded digits with the correction cue.";
        let mut l1_b = frozen["l1_finalist_a"].as_str().unwrap().to_string();
        for example in frozen["l1_finalist_a_examples"].as_array().unwrap() {
            l1_b = l1_b.replace(example.as_str().unwrap(), "");
        }
        assert_eq!(
            l1_b,
            APP_SYS_CORE
                .replace(EM_DASH_RULE, "")
                .replace(SPOKEN_STRUCTURE_RULE, "")
                .replace(CONTINUATION_CASING_RULE, "")
                .replace(NUMBER_LITERALS_CORRECTION_EXCEPTION, "")
                .replace(ASR_REPAIR_RULE, "")
                .replace(NUMBER_LITERALS_RULE, "")
                .replace(NAME_GARBLE_RULE, "")
                .replace(MEANING_WORDS_RULE, "")
                .replace(HOMOPHONE_GRAMMAR_RULE, "")
                .replace(OH_NO_CUE_AMENDED, OH_NO_CUE_FROZEN)
        );
        assert_eq!(frozen["l1_blocks"]["chat"], APP_SYS_CHAT);
        assert_eq!(frozen["l1_blocks"]["email"], APP_SYS_EMAIL);
        assert_eq!(frozen["l1_blocks"]["notes"], APP_SYS_NOTES);
        assert_eq!(frozen["l1_blocks"]["code"], APP_SYS_CODE);
        assert_eq!(frozen["l1_blocks"]["terminal"], APP_SYS_TERMINAL);
        // Post-bakeoff amendment (Adrian, 2026-07-15). ai_chat was the ONLY
        // app-kind block with no line-structure guidance — email has paragraph
        // breaks, notes has bullets — so a long dictation into Claude/ChatGPT
        // came back as one unbroken wall of text. frozen-prompts.json is left
        // UNTOUCHED: it is the evidence record of what the blind bakeoff actually
        // scored, and rewriting it to make a test pass would falsify that record.
        // Production = the frozen block PLUS this explicitly-recorded amendment.
        // Every other block still has to match the frozen bytes exactly, so this
        // guard keeps catching unintended prompt drift.
        const AI_CHAT_PARAGRAPH_AMENDMENT: &str = "\nBreak long dictation into paragraphs at the speaker's own topic shifts. Never merge two distinct requests into one, and never add headings or labels they did not speak.";
        assert_eq!(
            format!(
                "{}{}",
                frozen["l1_blocks"]["ai_chat"].as_str().unwrap(),
                AI_CHAT_PARAGRAPH_AMENDMENT
            ),
            APP_SYS_AI_CHAT
        );
        assert_eq!(
            frozen["l2_challenger_core"],
            PROMPT_SYS_CORE
                .replace(EM_DASH_RULE, "")
                .replace(ASR_REPAIR_RULE, "")
                .replace(HOMOPHONE_GRAMMAR_RULE, "")
                .replace(NUMBER_LITERALS_RULE, "")
        );
        assert_eq!(frozen["l2_blocks"]["code"], PROMPT_SYS_CODE);
        assert_eq!(frozen["l2_blocks"]["ai_chat"], PROMPT_SYS_AI_CHAT);
        assert_eq!(frozen["l2_blocks"]["terminal"], PROMPT_SYS_TERMINAL);
        assert_eq!(
            frozen["l3_challenger_core"],
            SUMMARY_SYS_CORE.replace(EM_DASH_RULE, "")
        );
        assert_eq!(frozen["l3_blocks"]["notes"], SUMMARY_SYS_NOTES);
        assert_eq!(frozen["l3_blocks"]["chat"], SUMMARY_SYS_CHAT);
        assert_eq!(frozen["l3_blocks"]["email"], SUMMARY_SYS_EMAIL);
        assert_eq!(frozen["l3_blocks"]["other"], SUMMARY_SYS_OTHER);
    }

    #[test]
    fn openrouter_payload_can_pin_provider() {
        with_env(
            &[("HEARDRIGHT_L3_OPENROUTER_PROVIDER_ONLY", Some("groq"))],
            || {
                let payload = build_payload(
                    &ProviderSpec {
                        provider: Provider::OpenRouter,
                        base_url: "https://openrouter.ai/api/v1".to_string(),
                        api_key: "test-key".to_string(),
                        model: "meta-llama/llama-3.3-70b-instruct".to_string(),
                    },
                    "send the in voice today",
                );
                assert_eq!(payload["provider"]["only"], json!(["groq"]));
                assert_eq!(payload["provider"]["allow_fallbacks"], false);
                assert_eq!(payload["provider"]["data_collection"], json!("deny"));
            },
        );
    }

    #[test]
    fn default_openrouter_payload_denies_data_collection_without_provider_pin() {
        with_env(&[], || {
            let payload = build_payload(
                &ProviderSpec {
                    provider: Provider::OpenRouter,
                    base_url: "https://openrouter.ai/api/v1".to_string(),
                    api_key: "test-key".to_string(),
                    model: L1_GROQ_PRIMARY.to_string(),
                },
                "send the in voice today",
            );
            assert!(payload["provider"].get("only").is_none());
            assert!(payload["provider"].get("allow_fallbacks").is_none());
            assert_eq!(payload["provider"]["data_collection"], json!("deny"));
        });
    }

    #[test]
    fn groq_gpt_oss_payload_pins_low_reasoning_effort() {
        // Without a pin, Groq gpt-oss defaults to medium reasoning and spends
        // the whole polish max_tokens budget on hidden reasoning (empty or
        // truncated content, finish_reason "length"). Verified live 2026-07-19.
        with_env(&[], || {
            let payload = build_payload(
                &ProviderSpec {
                    provider: Provider::Groq,
                    base_url: "https://api.groq.com/openai/v1".to_string(),
                    api_key: "test-key".to_string(),
                    model: L1_GROQ_SAME_PROVIDER_FALLBACK.to_string(),
                },
                "send the in voice today",
            );
            assert_eq!(payload["reasoning_effort"], json!("low"));
            assert!(payload.get("reasoning_format").is_none());
            // Reasoning shares the completion budget: without headroom even
            // "low" truncates long polishes (replay 2026-07-19: 7/30 at +0).
            let qwen_payload = build_payload(
                &ProviderSpec {
                    provider: Provider::Groq,
                    base_url: "https://api.groq.com/openai/v1".to_string(),
                    api_key: "test-key".to_string(),
                    model: L1_GROQ_PRIMARY.to_string(),
                },
                "send the in voice today",
            );
            let base = qwen_payload["max_tokens"].as_u64().unwrap();
            assert_eq!(payload["max_tokens"].as_u64().unwrap(), base + 768);
        });
    }

    #[test]
    fn every_groq_product_lane_uses_qwen36_as_primary_and_gpt_oss_as_same_provider_fallback() {
        // The Llama family is retired (Groq shutdown 2026-08-16);
        // the unreliable Groq gpt-oss-20b route is gone (factorial
        // 26/48 — circuit opened on malformed output). The only Groq
        // routes are now Qwen3.6 primary + gpt-oss-120b same-provider
        // fallback for every lane.
        assert_eq!(
            default_groq_primary_model(APP_PROMPT_VERSION),
            "qwen/qwen3.6-27b"
        );
        assert_eq!(
            default_groq_same_provider_fallback(APP_PROMPT_VERSION),
            "openai/gpt-oss-120b"
        );
        assert_eq!(
            default_groq_primary_model(PROMPT_POLISH_VERSION),
            "qwen/qwen3.6-27b"
        );
        assert_eq!(
            default_groq_same_provider_fallback(PROMPT_POLISH_VERSION),
            "openai/gpt-oss-120b"
        );
        assert_eq!(
            default_groq_primary_model(SUMMARY_PROMPT_VERSION),
            "qwen/qwen3.6-27b"
        );
        assert_eq!(
            default_groq_same_provider_fallback(SUMMARY_PROMPT_VERSION),
            "openai/gpt-oss-120b"
        );
    }

    #[test]
    fn default_roster_uses_conditional_ladder_when_both_keys_exist() {
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", None),
                ("GROQ_API_KEY", Some("groq-key")),
                ("CEREBRAS_API_KEY", Some("cerebras-key")),
                ("OPENROUTER_API_KEY", Some("openrouter-key")),
            ],
            || {
                // Both Groq + Cerebras: Groq Qwen primary, Cerebras
                // GPT-OSS-120B cross-provider, Groq GPT-OSS-120B
                // same-provider fallback. NVIDIA is opt-in via roster, so
                // it's absent here. OpenRouter is dormant and skipped
                // without the validation marker.
                let specs = provider_specs(APP_PROMPT_VERSION);
                assert_eq!(
                    specs
                        .iter()
                        .map(|spec| (spec.provider.as_str(), spec.model.as_str()))
                        .collect::<Vec<_>>(),
                    vec![
                        ("groq", L1_GROQ_PRIMARY),
                        ("cerebras", DEFAULT_CEREBRAS_MODEL),
                        ("groq", L1_GROQ_SAME_PROVIDER_FALLBACK),
                    ]
                );
                let specs = provider_specs(PROMPT_POLISH_VERSION);
                assert_eq!(
                    specs
                        .iter()
                        .map(|spec| (spec.provider.as_str(), spec.model.as_str()))
                        .collect::<Vec<_>>(),
                    vec![
                        ("groq", L2_GROQ_PRIMARY),
                        ("cerebras", DEFAULT_CEREBRAS_MODEL),
                        ("groq", L2_GROQ_SAME_PROVIDER_FALLBACK),
                    ]
                );
                let specs = provider_specs(SUMMARY_PROMPT_VERSION);
                assert_eq!(
                    specs
                        .iter()
                        .map(|spec| (spec.provider.as_str(), spec.model.as_str()))
                        .collect::<Vec<_>>(),
                    vec![
                        ("groq", L3_GROQ_PRIMARY),
                        ("cerebras", DEFAULT_CEREBRAS_MODEL),
                        ("groq", L3_GROQ_SAME_PROVIDER_FALLBACK),
                    ]
                );
            },
        );
    }

    #[test]
    fn default_roster_is_only_groq_when_only_groq_key_exists() {
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", None),
                ("GROQ_API_KEY", Some("groq-key")),
                ("CEREBRAS_API_KEY", None),
                ("OPENROUTER_API_KEY", None),
            ],
            || {
                let specs = provider_specs(APP_PROMPT_VERSION);
                assert_eq!(
                    specs
                        .iter()
                        .map(|spec| (spec.provider.as_str(), spec.model.as_str()))
                        .collect::<Vec<_>>(),
                    vec![
                        ("groq", L1_GROQ_PRIMARY),
                        ("groq", L1_GROQ_SAME_PROVIDER_FALLBACK),
                    ]
                );
            },
        );
    }

    #[test]
    fn default_roster_is_only_cerebras_when_only_cerebras_key_exists() {
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", None),
                ("GROQ_API_KEY", None),
                ("CEREBRAS_API_KEY", Some("cerebras-key")),
            ],
            || {
                let specs = provider_specs(APP_PROMPT_VERSION);
                assert_eq!(
                    specs
                        .iter()
                        .map(|spec| (spec.provider.as_str(), spec.model.as_str()))
                        .collect::<Vec<_>>(),
                    vec![("cerebras", DEFAULT_CEREBRAS_MODEL)]
                );
            },
        );
    }

    #[test]
    fn default_roster_is_empty_when_neither_key_exists() {
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", None),
                ("GROQ_API_KEY", None),
                ("CEREBRAS_API_KEY", None),
            ],
            || assert!(provider_specs(APP_PROMPT_VERSION).is_empty()),
        );
    }

    #[test]
    fn groq_only_fails_over_to_cerebras_when_only_cerebras_key_is_present() {
        // Inverse of the cerb-only test: only Cerebras means Cerebras is
        // primary/only, never reach for a Groq call we have no key for.
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", None),
                ("GROQ_API_KEY", None),
                ("CEREBRAS_API_KEY", Some("cerebras-key")),
                (
                    "HEARDRIGHT_L3_CEREBRAS_BASE_URL",
                    Some("http://127.0.0.1:9"),
                ),
                ("HEARDRIGHT_L3_TOTAL_TIMEOUT_MS", Some("500")),
                ("HEARDRIGHT_L3_PROVIDER_TIMEOUT_MS", Some("500")),
            ],
            || {
                assert_eq!(
                    provider_specs(APP_PROMPT_VERSION)
                        .iter()
                        .map(|spec| spec.provider.as_str())
                        .collect::<Vec<_>>(),
                    vec!["cerebras"]
                );
            },
        );
    }

    #[test]
    fn eval_roster_can_explicitly_include_extra_providers() {
        with_env(
            &[
                ("HEARDRIGHT_L3_ROSTER", Some("cerebras,nvidia")),
                ("CEREBRAS_API_KEY", Some("cerebras-key")),
                ("NVIDIA_API_KEY", Some("nvidia-key")),
            ],
            || {
                let specs = provider_specs(APP_PROMPT_VERSION);
                assert_eq!(
                    specs
                        .iter()
                        .map(|spec| spec.provider.as_str())
                        .collect::<Vec<_>>(),
                    vec!["cerebras", "nvidia"]
                );
            },
        );
    }

    include!("section05_more.rs");

    #[test]
    fn context_safety_scrubs_secrets_and_keeps_normal_context() {
        // Normal context passes through untouched.
        let clean = sanitize_context(&PolishContext {
            app_name: Some("Slack".to_string()),
            window_title: Some("Launch feedback".to_string()),
            field_text: Some("Draft so far: shipping tonight.".to_string()),
            vocabulary: vec!["OneClickDrive".to_string(), "Zephyr".to_string()],
            ..Default::default()
        });
        assert_eq!(clean.app_name.as_deref(), Some("Slack"));
        assert_eq!(clean.window_title.as_deref(), Some("Launch feedback"));
        assert_eq!(
            clean.field_text.as_deref(),
            Some("Draft so far: shipping tonight.")
        );
        assert_eq!(clean.vocabulary.len(), 2);

        // Browser window titles may include the signed-in account. Preserve
        // useful app/page context without sending that account address.
        let gmail = sanitize_context(&PolishContext {
            app_name: Some("msedge.exe".to_string()),
            window_title: Some("Draft - adrdsouza@gmail.com - Gmail - Personal".to_string()),
            ..Default::default()
        });
        assert_eq!(
            gmail.window_title.as_deref(),
            Some("Draft - [email] - Gmail - Personal")
        );

        // Password-manager app: ALL machine-captured context dropped.
        let pw = sanitize_context(&PolishContext {
            app_name: Some("1Password 8".to_string()),
            window_title: Some("Chase Bank — vault".to_string()),
            field_text: Some("hunter2".to_string()),
            selected_text: Some("hunter2".to_string()),
            field_context_available: true,
            vocabulary: Vec::new(),
            writing_region: None,
            sound_alikes: Vec::new(),
        });
        assert_eq!(pw.app_name, None);
        assert_eq!(pw.window_title, None);
        assert_eq!(pw.field_text, None);
        assert_eq!(pw.selected_text, None);
        assert!(!pw.field_context_available);

        // Secret-shaped field text dropped even in a normal app.
        let secret_field = sanitize_context(&PolishContext {
            app_name: Some("TextEdit".to_string()),
            field_text: Some("GROQ_API_KEY=gsk_abc12345".to_string()),
            ..Default::default()
        });
        assert_eq!(secret_field.field_text, None);
        assert_eq!(secret_field.app_name.as_deref(), Some("TextEdit"));

        // Secret-shaped window titles dropped; app kept.
        for title in [
            "password: hunter22222",
            "GROQ_API_KEY=gsk_abc12345",
            "sk-ant-abcdefghijklmnop settings",
            "Card 4539 1488 0343 6467 saved", // Luhn-valid test number
        ] {
            let out = sanitize_context(&PolishContext {
                app_name: Some("TextEdit".to_string()),
                window_title: Some(title.to_string()),
                ..Default::default()
            });
            assert_eq!(out.window_title, None, "should drop title: {title}");
            assert_eq!(out.app_name.as_deref(), Some("TextEdit"));
        }

        // Secret-shaped vocab terms dropped, normal ones kept.
        let vocab = sanitize_context(&PolishContext {
            vocabulary: vec!["HeardRight".to_string(), "ghp_0123456789abcdef".to_string()],
            ..Default::default()
        });
        assert_eq!(vocab.vocabulary, vec!["HeardRight".to_string()]);

        // Ordinary numbers do NOT trip the card check (too short / not Luhn).
        assert!(!looks_secret("Invoice 4250 for OneClickDrive"));
        assert!(!looks_secret("Meeting notes 2026-07-02"));
    }

    #[test]
    fn user_vocabulary_block_renders_and_caps() {
        assert_eq!(user_vocabulary_block(&[]), "");
        let block = user_vocabulary_block(&["OneClickDrive".to_string(), "Zephyr".to_string()]);
        assert!(block.starts_with('\n'));
        assert!(block.contains("OneClickDrive, Zephyr"));
        // Over-long terms are skipped.
        let long = "x".repeat(80);
        assert_eq!(user_vocabulary_block(&[long]), "");
    }

    #[test]
    fn vocab_v2_block_renders_sound_alike_corrections_with_context_gate() {
        // Empty input: empty output.
        assert_eq!(user_vocabulary_block_v2(&[], &[]), "");
        // No aliases: same as the spelling-only block (no "Known mishearings"
        // header rendered).
        let block = user_vocabulary_block_v2(&["HeardRight".to_string()], &[]);
        assert!(block.contains("HeardRight"));
        assert!(!block.contains("Known mishearings"));
        // With aliases: opt-in context-gated block appears and renders the
        // mapping.
        let block = user_vocabulary_block_v2(
            &["Wispr Flow".to_string()],
            &[("Wispr Flow".to_string(), vec!["whisper flow".to_string()])],
        );
        assert!(block.contains("Known mishearings"));
        assert!(block.contains("\"whisper flow\" -> \"Wispr Flow\""));
        // Alias equal to term is skipped — it would self-reference.
        let block = user_vocabulary_block_v2(
            &["Wispr".to_string()],
            &[("Wispr".to_string(), vec!["wispr".to_string()])],
        );
        assert!(!block.contains("\"wispr\" ->"));
    }
}
