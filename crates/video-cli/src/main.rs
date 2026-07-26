use clap::{Args, Parser, Subcommand};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "videoctl",
    version,
    about = "CutRight's deterministic video control plane"
)]
struct Cli {
    #[arg(long, global = true, help = "Do not write project or media state")]
    dry_run: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Doctor,
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    Ingest(IngestArgs),
    Transcribe(TranscribeArgs),
    Bench {
        #[command(subcommand)]
        command: BenchCommand,
    },
    Analyze {
        #[command(subcommand)]
        command: AnalyzeCommand,
    },
    Reframe {
        #[command(subcommand)]
        command: ReframeCommand,
    },
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    Edit {
        #[command(subcommand)]
        command: EditCommand,
    },
    Review(PathCommand),
    #[command(name = "transcript")]
    Transcript {
        #[command(subcommand)]
        command: TranscriptCommand,
    },
    Shorts(ShortsArgs),
    Finish {
        #[command(subcommand)]
        command: FinishCommand,
    },
    Slot(SlotArgs),
    Render {
        #[command(subcommand)]
        command: RenderCommand,
    },
    Qa(PathCommand),
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    Init { folder: PathBuf },
}

#[derive(Debug, Args)]
struct PathCommand {
    project: PathBuf,
}

#[derive(Debug, Args)]
struct IngestArgs {
    project: PathBuf,
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
    sources: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct TranscribeArgs {
    project: PathBuf,
    #[arg(long, default_value = "heardright")]
    provider: String,
}

#[derive(Debug, Subcommand)]
enum BenchCommand {
    Transcribe {
        project: PathBuf,
        #[arg(long, default_value = "heardright")]
        primary: String,
        #[arg(long, default_value = "whisperx")]
        verifier: String,
        #[arg(long, default_value_t = 20)]
        boundaries: usize,
        #[arg(long, default_value_t = 40)]
        padding_ms: i64,
    },
}

#[derive(Debug, Subcommand)]
enum AnalyzeCommand {
    Local(PathCommand),
    Cloud {
        project: PathBuf,
        #[arg(long)]
        provider: String,
    },
}

#[derive(Debug, Subcommand)]
enum ReframeCommand {
    Plan(PathCommand),
}

#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    Build(PathCommand),
}

#[derive(Debug, Subcommand)]
enum EditCommand {
    Candidates(PathCommand),
    Validate(PathCommand),
    Render {
        project: PathBuf,
        #[arg(long)]
        variant: String,
    },
}

#[derive(Debug, Subcommand)]
enum TranscriptCommand {
    Remap(PathCommand),
}

#[derive(Debug, Args)]
struct ShortsArgs {
    #[command(subcommand)]
    command: ShortsCommand,
}

#[derive(Debug, Subcommand)]
enum ShortsCommand {
    Propose {
        project: PathBuf,
        #[arg(long, default_value_t = 4)]
        count: u8,
    },
}

#[derive(Debug, Subcommand)]
enum FinishCommand {
    Validate(PathCommand),
}

#[derive(Debug, Args)]
struct SlotArgs {
    #[command(subcommand)]
    command: SlotCommand,
}

#[derive(Debug, Subcommand)]
enum SlotCommand {
    Render { project: PathBuf, slot_id: String },
}

#[derive(Debug, Subcommand)]
enum RenderCommand {
    Preview(PathCommand),
    Final {
        project: PathBuf,
        #[arg(long)]
        preset: String,
    },
}

#[derive(Debug, Subcommand)]
enum PackageCommand {
    Social(PathCommand),
}

#[derive(Debug, Subcommand)]
enum ExportCommand {
    Otio(PathCommand),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(value) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).expect("JSON serialization cannot fail")
            );
            ExitCode::SUCCESS
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
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<Value, String> {
    match cli.command {
        Command::Doctor => Ok(doctor()),
        Command::Project {
            command: ProjectCommand::Init { folder },
        } => video_project::init_project(&folder, cli.dry_run)
            .map(|result| json!({ "event": "project.init", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Ingest(args) => {
            video_project::ingest_sources(&args.project, &args.sources, cli.dry_run)
                .map(|result| json!({ "event": "ingest", "result": result }))
                .map_err(|error| error.to_string())
        }
        Command::Transcribe(args) => {
            video_project::transcribe_project(&args.project, &args.provider, cli.dry_run)
                .map(|result| json!({ "event": "transcribe", "result": result }))
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
        .map(|result| json!({ "event": "bench.transcribe", "result": result }))
        .map_err(|error| error.to_string()),
        Command::Analyze {
            command: AnalyzeCommand::Local(args),
        } => video_project::analyze_local(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "analyze.local", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Reframe {
            command: ReframeCommand::Plan(args),
        } => video_project::reframe_plan(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "reframe.plan", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Evidence {
            command: EvidenceCommand::Build(args),
        } => video_project::evidence_build(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "evidence.build", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Edit {
            command: EditCommand::Candidates(args),
        } => video_project::build_candidates(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "edit.candidates", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Edit {
            command: EditCommand::Validate(args),
        } => video_project::validate_edit(&args.project)
            .map(|result| json!({ "event": "edit.validate", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Edit {
            command: EditCommand::Render { project, variant },
        } => video_project::build_cut_plan(&project, &variant, cli.dry_run)
            .and_then(|_| video_project::compile_timeline(&project, cli.dry_run))
            .and_then(|_| video_project::render_edit(&project, &variant, cli.dry_run))
            .map(|result| json!({ "event": "edit.render", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Transcript {
            command: TranscriptCommand::Remap(args),
        } => video_project::remap_transcript(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "transcript.remap", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Render {
            command: RenderCommand::Final { project, preset },
        } => video_project::render_final(&project, &preset, cli.dry_run)
            .map(|result| json!({ "event": "render.final", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Render {
            command: RenderCommand::Preview(args),
        } => video_project::build_cut_plan(&args.project, "tight", cli.dry_run)
            .and_then(|_| video_project::compile_timeline(&args.project, cli.dry_run))
            .and_then(|_| video_project::render_edit(&args.project, "tight", cli.dry_run))
            .map(|result| json!({ "event": "render.preview", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Qa(args) => video_project::qa_run(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "qa.run", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Shorts(args) => match args.command {
            ShortsCommand::Propose { project, count } => {
                video_project::propose_shorts(&project, count, cli.dry_run)
                    .map(|result| json!({ "event": "shorts.propose", "result": result }))
                    .map_err(|error| error.to_string())
            }
        },
        Command::Finish {
            command: FinishCommand::Validate(args),
        } => video_project::finish_validate(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "finish.validate", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Slot(args) => match args.command {
            SlotCommand::Render { project, slot_id } => {
                video_project::render_slot(&project, &slot_id, cli.dry_run)
                    .map(|result| json!({ "event": "slot.render", "result": result }))
                    .map_err(|error| error.to_string())
            }
        },
        Command::Package {
            command: PackageCommand::Social(args),
        } => video_project::package_social(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "package.social", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Export {
            command: ExportCommand::Otio(args),
        } => video_project::export_otio(&args.project, cli.dry_run)
            .map(|result| json!({ "event": "export.otio", "result": result }))
            .map_err(|error| error.to_string()),
        Command::Review(args) => Ok(json!({
            "event": "review.open",
            "status": "ready",
            "project": args.project,
            "artifacts": ["edit/candidates.json", "edit/cut-plan.json", "edit/timeline.json"]
        })),
        command => Ok(not_implemented(command_name(&command), cli.dry_run)),
    }
}

fn doctor() -> Value {
    let checks = [
        command_check("ffmpeg"),
        command_check("ffprobe"),
        json!({ "name": "source_policy", "status": "ok", "value": "immutable" }),
        json!({ "name": "cloud_default", "status": "ok", "value": "disabled" }),
    ];
    let status = if checks.iter().all(|check| check["status"] == "ok") {
        "ok"
    } else {
        "error"
    };
    json!({ "event": "doctor", "status": status, "checks": checks })
}

fn command_check(name: &str) -> Value {
    let status = std::process::Command::new(name)
        .arg("-version")
        .output()
        .is_ok();
    json!({ "name": name, "status": if status { "ok" } else { "missing" } })
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
        Command::Review(_) => "review open",
        Command::Transcript { .. } => "transcript remap",
        Command::Shorts(_) => "shorts propose",
        Command::Finish { .. } => "finish validate",
        Command::Slot(_) => "slot render",
        Command::Render { .. } => "render",
        Command::Qa(_) => "qa run",
        Command::Package { .. } => "package social",
        Command::Export { .. } => "export otio",
        Command::Doctor | Command::Project { .. } => "unknown",
    }
    .to_string()
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
