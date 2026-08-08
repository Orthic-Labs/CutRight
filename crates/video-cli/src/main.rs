mod agent;
mod agent_qualify;
mod apply;
mod capabilities;
mod cli;
mod doctor;
mod terminal;

use clap::Parser;
use cli::{
    AgentCommand, AgentTerminalCommand, AnalyzeCommand, BenchCommand, CleanMachineSampleArgs, Cli,
    CloudCommand, Command, EditCommand, EvidenceCommand, ExportCommand, FinishCommand,
    PackageCommand, PreferencesCommand, ProjectCommand, ReceiptsCommand, ReframeCommand,
    RenderCommand, ReviewCommand, ShortsCommand, SlotCommand, TranscriptCommand,
};
use doctor::DoctorOutcome;
use serde_json::{json, Value};
use std::process::ExitCode;

/// CLI exit-code table (plan §10.8). `status: "error"` and every other
/// nonzero path each get a stable, distinct code so callers (CI, release
/// gates, the Studio shell-out) can branch on *why* videoctl failed instead
/// of parsing stderr text.
///
/// | code | meaning                                                        |
/// |------|-----------------------------------------------------------------|
/// | 0    | success (`status` absent or not one of the failure sentinels)   |
/// | 1    | `status: "error"` — a command ran and reported a domain error   |
/// | 3    | `status: "not_implemented"` — command is behind the frozen CLI  |
/// |      | contract but not yet built                                     |
/// | 4    | invalid command/config — clap usage error, or a doctor profile  |
/// |      | name clap accepted but this binary could not otherwise resolve  |
/// | 5    | `videoctl doctor` ran but a required check was not `ok`          |
/// | 6    | `videoctl receipts verify` found a receipt whose input/output    |
///       | bindings no longer match the bytes on disk                       |
///
/// JSON always goes to stdout; this module never writes diagnostic text to
/// stdout outside of the single final JSON document (clap's own `--help`/
/// `--version` output is the one exception, matching normal CLI UX).
const EXIT_OK: u8 = 0;
const EXIT_ERROR: u8 = 1;
const EXIT_NOT_IMPLEMENTED: u8 = 3;
const EXIT_INVALID: u8 = 4;
const EXIT_DOCTOR_FAIL: u8 = 5;
const EXIT_RECEIPTS_FAIL: u8 = 6;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            use clap::error::ErrorKind;
            // `--help`/`--version` are not usage errors: print clap's own
            // text as-is (matching normal CLI UX) and exit 0.
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                print!("{error}");
                return ExitCode::from(EXIT_OK);
            }
            // Every other parse failure is stable-code "invalid command/config"
            // (plan §10.8). Usage text goes to stderr; the JSON summary still
            // goes to stdout so scripted callers never have to branch on
            // whether a failure was a parse error or a domain error.
            eprint!("{error}");
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "event": "error",
                    "status": "error",
                    "error_kind": "invalid_command",
                    "error": error.to_string(),
                }))
                .expect("JSON serialization cannot fail")
            );
            return ExitCode::from(EXIT_INVALID);
        }
    };

    // The capabilities command writes its own single JSON document to stdout
    // and MUST NOT be wrapped in a second "event"/"status" envelope. Handle
    // it before the generic run() path so the output stays byte-stable.
    if let Command::Capabilities { command } = &cli.command {
        return match capabilities::run(command) {
            ExitCode::SUCCESS => ExitCode::from(EXIT_OK),
            _ => ExitCode::from(EXIT_ERROR),
        };
    }
    // The apply command also writes its own single JSON document (the
    // executor report) and exits with its own table of codes. Bypass the
    // generic run() path so the output stays byte-stable.
    if let Command::Apply { args } = &cli.command {
        return apply::run(args);
    }
    match run(cli) {
        Ok(Outcome::Value(value)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("JSON serialization cannot fail")
            );
            ExitCode::from(EXIT_OK)
        }
        Ok(Outcome::NotImplemented(value)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("JSON serialization cannot fail")
            );
            ExitCode::from(EXIT_NOT_IMPLEMENTED)
        }
        Ok(Outcome::Doctor(value, doctor_outcome)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("JSON serialization cannot fail")
            );
            match doctor_outcome {
                DoctorOutcome::Ready => ExitCode::from(EXIT_OK),
                DoctorOutcome::NotReady => ExitCode::from(EXIT_DOCTOR_FAIL),
            }
        }
        Ok(Outcome::Receipts(value, all_passed)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("JSON serialization cannot fail")
            );
            if all_passed {
                ExitCode::from(EXIT_OK)
            } else {
                ExitCode::from(EXIT_RECEIPTS_FAIL)
            }
        }
        Ok(Outcome::CleanMachine(value, passed)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("JSON serialization cannot fail")
            );
            ExitCode::from(if passed { EXIT_OK } else { EXIT_ERROR })
        }
        Ok(Outcome::Qualification(value, passed)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("JSON serialization cannot fail")
            );
            ExitCode::from(if passed { EXIT_OK } else { EXIT_ERROR })
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "event": "error",
                    "status": "error",
                    "error": error,
                }))
                .expect("JSON serialization cannot fail")
            );
            ExitCode::from(EXIT_ERROR)
        }
    }
}

/// A successful `run()` still needs to tell `main` which exit-code branch
/// applies: a plain result, an explicit `not_implemented` stub, or a
/// doctor report whose own required-check state decides the code.
enum Outcome {
    Value(Value),
    NotImplemented(Value),
    Doctor(Value, DoctorOutcome),
    Receipts(Value, bool),
    CleanMachine(Value, bool),
    Qualification(Value, bool),
}

fn run(cli: Cli) -> Result<Outcome, String> {
    match cli.command {
        Command::Doctor(args) => {
            let (report, outcome) =
                doctor::run(args.profile, args.strict, args.write_receipt.as_deref());
            Ok(Outcome::Doctor(report, outcome))
        }
        Command::CleanMachineSample(args) => clean_machine_sample(args),
        Command::Agent { command } => run_agent(command),
        Command::Project {
            command: ProjectCommand::Init { folder },
        } => video_project::init_project(&folder, cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "project.init", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Project {
            command: ProjectCommand::Migrate { folder },
        } => video_project::migrate_project(&folder)
            .map(|result| Outcome::Value(json!({ "event": "project.migrate", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Ingest(args) => {
            video_project::ingest_sources(&args.project, &args.sources, cli.dry_run)
                .map(|result| Outcome::Value(json!({ "event": "ingest", "result": result })))
                .map_err(|error| error.to_string())
        }
        Command::Transcribe(args) => {
            video_project::transcribe_project(&args.project, &args.provider, cli.dry_run)
                .map(|result| Outcome::Value(json!({ "event": "transcribe", "result": result })))
                .map_err(|error| error.to_string())
        }
        Command::Bench {
            command:
                BenchCommand::Transcribe {
                    project,
                    primary,
                    verifier,
                    boundaries,
                    padding_ms,
                },
        } => video_project::bench_transcribe(
            &project,
            &primary,
            &verifier,
            boundaries,
            padding_ms,
            cli.dry_run,
        )
        .map(|result| Outcome::Value(json!({ "event": "bench.transcribe", "result": result })))
        .map_err(|error| error.to_string()),
        Command::Analyze {
            command: AnalyzeCommand::Local(args),
        } => video_project::analyze_local(&args.project, cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "analyze.local", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Analyze {
            command: AnalyzeCommand::Cloud(args),
        } => video_project::CloudCapability::parse(&args.capability)
            .map_err(|error| error.to_string())
            .and_then(|capability| {
                let provider = video_project::resolve_provider(&args.provider);
                video_project::analyze_cloud(
                    &args.project,
                    provider.as_ref(),
                    capability,
                    args.target.as_deref(),
                    args.use_source,
                    cli.dry_run,
                )
                .map_err(|error| error.to_string())
            })
            .map(|result| Outcome::Value(json!({ "event": "analyze.cloud", "result": result }))),
        Command::Cloud {
            command: CloudCommand::Consent { project, enable },
        } => video_project::set_cloud_consent(&project, enable, cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "cloud.consent", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Cloud {
            command: CloudCommand::Budget { project, usd },
        } => video_project::set_cloud_budget(&project, usd, cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "cloud.budget", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Cloud {
            command: CloudCommand::Delete(args),
        } => video_project::delete_cloud_retention(&args.project, cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "cloud.delete", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Reframe {
            command: ReframeCommand::Plan(args),
        } => video_project::reframe_plan(&args.project, args.variant.as_deref(), cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "reframe.plan", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Evidence {
            command: EvidenceCommand::Build(args),
        } => video_project::evidence_build(&args.project, cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "evidence.build", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Edit {
            command: EditCommand::Candidates(args),
        } => video_project::build_candidates(&args.project, cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "edit.candidates", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Edit {
            command: EditCommand::Validate(args),
        } => video_project::validate_edit(&args.project, args.variant.as_deref())
            .map(|result| Outcome::Value(json!({ "event": "edit.validate", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Edit {
            command: EditCommand::Render { project, variant },
        } => video_project::build_cut_plan(&project, &variant, cli.dry_run)
            .and_then(|_| video_project::compile_timeline(&project, &variant, cli.dry_run))
            .and_then(|_| video_project::render_edit(&project, &variant, cli.dry_run))
            .and_then(|_| {
                video_project::remap_transcript_for_variant(&project, &variant, cli.dry_run)
            })
            .map(|result| Outcome::Value(json!({ "event": "edit.render", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Transcript {
            command: TranscriptCommand::Remap { project, variant },
        } => {
            video_project::remap_transcript_with_variant(&project, variant.as_deref(), cli.dry_run)
                .map(|result| {
                    Outcome::Value(json!({ "event": "transcript.remap", "result": result }))
                })
                .map_err(|error| error.to_string())
        }
        Command::Render {
            command:
                RenderCommand::Final {
                    project,
                    preset,
                    variant,
                },
        } => video_project::render_final(&project, &preset, variant.as_deref(), cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "render.final", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Render {
            command: RenderCommand::Preview(args),
        } => video_project::build_cut_plan(&args.project, "tight", cli.dry_run)
            .and_then(|_| video_project::compile_timeline(&args.project, "tight", cli.dry_run))
            .and_then(|_| video_project::render_edit(&args.project, "tight", cli.dry_run))
            .map(|result| Outcome::Value(json!({ "event": "render.preview", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Qa(args) => video_project::qa_run(
            &args.project,
            args.variant.as_deref(),
            &args.preset,
            cli.dry_run,
        )
        .map(|result| Outcome::Value(json!({ "event": "qa.run", "result": result })))
        .map_err(|error| error.to_string()),
        Command::Shorts(args) => match args.command {
            ShortsCommand::Propose { project, count } => {
                video_project::propose_shorts(&project, count, cli.dry_run)
                    .map(|result| {
                        Outcome::Value(json!({ "event": "shorts.propose", "result": result }))
                    })
                    .map_err(|error| error.to_string())
            }
        },
        Command::Finish {
            command: FinishCommand::Validate(args),
        } => video_project::finish_validate(&args.project, args.variant.as_deref(), cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "finish.validate", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Slot(args) => match args.command {
            SlotCommand::Render { project, slot_id } => {
                video_project::render_slot(&project, &slot_id, cli.dry_run)
                    .map(|result| {
                        Outcome::Value(json!({ "event": "slot.render", "result": result }))
                    })
                    .map_err(|error| error.to_string())
            }
        },
        Command::Package {
            command: PackageCommand::Social(args),
        } => video_project::package_social(&args.project, cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "package.social", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Export {
            command: ExportCommand::Otio(args),
        } => video_project::export_otio(&args.project, args.variant.as_deref(), cli.dry_run)
            .map(|result| Outcome::Value(json!({ "event": "export.otio", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Receipts {
            command: ReceiptsCommand::Verify(args),
        } => video_project::verify_receipts(&args.project)
            .map(|report| {
                let all_passed = report.status == "pass";
                Outcome::Receipts(
                    json!({ "event": "receipts.verify", "result": report }),
                    all_passed,
                )
            })
            .map_err(|error| error.to_string()),
        Command::Review {
            command: ReviewCommand::Open(args),
        } => Ok(Outcome::Value(json!({
            "event": "review.open",
            "status": "ready",
            "project": args.project,
            "artifacts": ["edit/candidates.json", "edit/cut-plan.json", "edit/timeline.json"]
        }))),
        Command::Review {
            command: ReviewCommand::Select { project, variant },
        } => video_project::select_variant(&project, &variant, "cli")
            .map(|result| Outcome::Value(json!({ "event": "review.select", "result": result })))
            .map_err(|error| error.to_string()),
        Command::Preferences {
            command: PreferencesCommand::Recommend { projects, out },
        } => {
            let output_path = match (out, projects.len()) {
                (Some(path), _) => Ok(path),
                (None, 1) => Ok(projects[0].join("feedback/preferences/recommendations.json")),
                (None, _) => {
                    Err("--out is required when more than one project is given".to_string())
                }
            };
            output_path.and_then(|output_path| {
                video_project::recommend_preferences(&projects, &output_path, cli.dry_run)
                    .map(|result| {
                        Outcome::Value(
                            json!({ "event": "preferences.recommend", "result": result }),
                        )
                    })
                    .map_err(|error| error.to_string())
            })
        }
        // Every `Command` variant is matched explicitly above as of this
        // change (cloud analysis was the last stub the frozen CLI contract
        // reserved exit code 3 for). This arm, `command_name`, and
        // `not_implemented` stay in place on purpose — unreachable today,
        // not dead: a future `Command` variant added without its own arm
        // above still falls through here and reports a clean
        // `status: "not_implemented"` / exit 3 instead of a match error.
        #[allow(unreachable_patterns)]
        command => Ok(Outcome::NotImplemented(not_implemented(
            command_name(&command),
            cli.dry_run,
        ))),
    }
}

fn command_name(command: &Command) -> String {
    match command {
        Command::Ingest(_) => "ingest",
        Command::Transcribe(_) => "transcribe",
        Command::Bench { .. } => "bench transcribe",
        Command::Analyze { .. } => "analyze",
        Command::Reframe { .. } => "reframe plan",
        Command::Evidence { .. } => "evidence build",
        Command::Edit { .. } => "edit",
        Command::Review { .. } => "review",
        Command::Transcript { .. } => "transcript remap",
        Command::Shorts(_) => "shorts propose",
        Command::Finish { .. } => "finish validate",
        Command::Slot(_) => "slot render",
        Command::Render { .. } => "render",
        Command::Qa(_) => "qa run",
        Command::Package { .. } => "package social",
        Command::Export { .. } => "export otio",
        Command::Receipts { .. } => "receipts verify",
        Command::Preferences { .. } => "preferences recommend",
        Command::Cloud { .. } => "cloud",
        Command::Capabilities { .. } => "capabilities list",
        Command::CleanMachineSample(_) => "clean-machine-sample",
        Command::Apply { .. } => "apply",
        Command::Agent { .. } => "agent",
        Command::Doctor(_) | Command::Project { .. } => "unknown",
    }
    .to_string()
}

fn run_agent(command: AgentCommand) -> Result<Outcome, String> {
    match command {
        AgentCommand::Integrate(args) => {
            let provider = video_cli_provider(&args.provider)?;
            agent::run(agent::AgentCommand {
                provider,
                binary: args.binary,
                config: args.config,
                remove: false,
            })
            .map(Outcome::Value)
        }
        AgentCommand::Status(args) => {
            let provider = video_cli_provider(&args.provider)?;
            match args.config {
                Some(config) => agent::status(provider, &config).map(Outcome::Value),
                None => Ok(Outcome::Value(json!({
                    "event": "agent.status",
                    "provider": provider,
                    "registered": false,
                    "status": "unconfigured"
                }))),
            }
        }
        AgentCommand::Remove(args) => {
            let provider = video_cli_provider(&args.provider)?;
            agent::run(agent::AgentCommand {
                provider,
                binary: args.binary,
                config: args.config,
                remove: true,
            })
            .map(Outcome::Value)
        }
        AgentCommand::Qualify(args) => {
            if !args.all {
                return Err("agent qualify requires --all".into());
            }
            agent_qualify::run_all().map(|(value, passed)| Outcome::Qualification(value, passed))
        }
        AgentCommand::Terminal {
            command: AgentTerminalCommand::Attach(args),
        } => {
            let request = terminal::AttachRequest {
                session_id: args.session_id,
                attach_token: String::new(),
                columns: 120,
                rows: 40,
            };
            let mut view = terminal::TerminalView::default();
            view.apply(terminal::parse_terminal_event(&[]));
            let event_kinds = [
                terminal_event_name(&terminal::TerminalEvent::Bytes(Vec::new())),
                terminal_event_name(&terminal::TerminalEvent::PromptVisible),
                terminal_event_name(&terminal::TerminalEvent::Exit(None)),
                terminal_event_name(&terminal::TerminalEvent::Error(String::new())),
            ];
            Ok(Outcome::Value(json!({
                "event": "agent.terminal.attach",
                "status": "attach_requested",
                "session_id": request.session_id,
                "argv": request.argv(),
                "command": terminal::ATTACH_COMMAND,
                "columns": request.columns,
                "rows": request.rows,
                "initial_bytes": view.presentation_bytes(),
                "prompt_visible": view.prompt_visible,
                "event_kinds": event_kinds,
                "presentation_only": true
            })))
        }
    }
}

fn video_cli_provider(value: &str) -> Result<agent::Provider, String> {
    agent::Provider::parse(value).map_err(|error| error.to_string())
}

fn terminal_event_name(event: &terminal::TerminalEvent) -> &'static str {
    match event {
        terminal::TerminalEvent::Bytes(_) => "bytes",
        terminal::TerminalEvent::PromptVisible => "prompt_visible",
        terminal::TerminalEvent::Exit(_) => "exit",
        terminal::TerminalEvent::Error(_) => "error",
    }
}

fn clean_machine_sample(args: CleanMachineSampleArgs) -> Result<Outcome, String> {
    let pack_ids = if args.pack_ids.is_empty() {
        std::env::var("CUTRIGHT_PACK_IDS")
            .ok()
            .map(|ids| {
                ids.split(',')
                    .filter(|id| !id.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    } else {
        args.pack_ids
    };
    let network_denied = args.network_deny
        || matches!(
            std::env::var("CUTRIGHT_NETWORK_POLICY").as_deref(),
            Ok("deny")
        );
    let report = video_project::clean_machine_sample(
        &args.project,
        args.sample.as_deref(),
        args.lane.as_deref(),
        pack_ids,
        network_denied,
        &args.lifecycle,
    )
    .map_err(|error| error.to_string())?;
    let passed = report.all_requested_supported_and_passed();
    Ok(Outcome::CleanMachine(
        serde_json::to_value(report).map_err(|error| error.to_string())?,
        passed,
    ))
}

fn not_implemented(command: String, dry_run: bool) -> Value {
    json!({
        "event": "command",
        "command": command,
        "status": "not_implemented",
        "dry_run": dry_run,
        "phase": 0,
        "message": "The command is part of the frozen CLI contract and lands in a later phase."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `Command` variant is matched explicitly in `run()` as of the
    /// cloud-analysis change, so no CLI invocation can reach the
    /// `not_implemented` fallback any more — the black-box integration test
    /// that used to exercise it (`analyze cloud`) now asserts real
    /// behavior instead. This unit test keeps the exit-code-3 contract
    /// (plan §10.8) covered directly: the JSON shape `main()` prints, and
    /// the exit code it maps to, for whichever future command eventually
    /// falls through to this path again.
    #[test]
    fn not_implemented_reports_the_documented_shape_and_exit_code() {
        let value = not_implemented("future command".to_string(), false);
        assert_eq!(value["status"], "not_implemented");
        assert_eq!(value["command"], "future command");
        assert_eq!(value["dry_run"], false);
        assert_eq!(EXIT_NOT_IMPLEMENTED, 3);
    }
}
