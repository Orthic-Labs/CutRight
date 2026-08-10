use crate::doctor::DoctorProfile;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "videoctl",
    version,
    about = "CutRight's deterministic video control plane"
)]
pub struct Cli {
    #[arg(long, global = true, help = "Do not write project or media state")]
    pub dry_run: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long, value_enum, default_value_t = DoctorProfile::Core)]
    pub profile: DoctorProfile,
    #[arg(long, help = "Also fail on degraded (non-required) checks")]
    pub strict: bool,
    #[arg(
        long,
        help = "Write a timestamped, hashable machine-readiness receipt to this path"
    )]
    pub write_receipt: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Doctor(DoctorArgs),
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
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
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
    Qa(QaArgs),
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    Receipts {
        #[command(subcommand)]
        command: ReceiptsCommand,
    },
    Preferences {
        #[command(subcommand)]
        command: PreferencesCommand,
    },
    Cloud {
        #[command(subcommand)]
        command: CloudCommand,
    },
    /// Inspect the canonical capability registry owned by Lane P-B
    /// (CR-V2-B2-014). Read-only; never mutates a project.
    Capabilities {
        #[command(subcommand)]
        command: CapabilitiesCommand,
    },
    /// Run one bundled project through locally available lifecycle checks.
    CleanMachineSample(CleanMachineSampleArgs),
    /// Drive the single Book 2 ActionExecutor with a
    /// `cutright.action_batch/v1` from stdin/file. The CLI never
    /// duplicates executor logic; it only marshals JSON (B2-023).
    Apply {
        #[command(flatten)]
        args: ApplyArgs,
    },
    /// Register or inspect CutRight's provider-native MCP entry.
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    Integrate(AgentIntegrateArgs),
    Status(AgentStatusArgs),
    Remove(AgentRemoveArgs),
    Qualify(AgentQualifyArgs),
    Terminal {
        #[command(subcommand)]
        command: AgentTerminalCommand,
    },
}

#[derive(Debug, Args)]
pub struct AgentQualifyArgs {
    #[arg(long)]
    pub all: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum AgentTerminalCommand {
    Attach(AgentTerminalAttachArgs),
}

#[derive(Debug, Args)]
pub struct AgentTerminalAttachArgs {
    pub session_id: String,
}

#[derive(Debug, Args)]
pub struct AgentIntegrateArgs {
    #[arg(long)]
    pub provider: String,
    #[arg(long)]
    pub config: PathBuf,
    #[arg(long)]
    pub binary: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AgentStatusArgs {
    #[arg(long, default_value = "claude-code")]
    pub provider: String,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AgentRemoveArgs {
    #[arg(long)]
    pub provider: String,
    #[arg(long)]
    pub config: PathBuf,
    #[arg(long)]
    pub binary: PathBuf,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CleanMachineSampleArgs {
    /// Bundled project descriptor (`project.json`). It is copied before any check writes.
    pub project: PathBuf,
    #[arg(long)]
    pub sample: Option<String>,
    #[arg(long)]
    pub lane: Option<String>,
    #[arg(long = "pack-id", value_delimiter = ',')]
    pub pack_ids: Vec<String>,
    #[arg(long)]
    pub network_deny: bool,
    #[arg(long, value_delimiter = ',')]
    pub lifecycle: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    Init { folder: PathBuf },
    Migrate { folder: PathBuf },
}

#[derive(Debug, Args)]
pub struct PathCommand {
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct VariantPathCommand {
    pub project: PathBuf,
    #[arg(
        long,
        help = "tight|natural; defaults to the reviewed selection, else natural"
    )]
    pub variant: Option<String>,
}

#[derive(Debug, Args)]
pub struct QaArgs {
    pub project: PathBuf,
    #[arg(
        long,
        help = "tight|natural; defaults to the reviewed selection, else natural"
    )]
    pub variant: Option<String>,
    #[arg(long, default_value = "youtube")]
    pub preset: String,
}

#[derive(Debug, Args)]
pub struct IngestArgs {
    pub project: PathBuf,
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
    pub sources: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub struct TranscribeArgs {
    pub project: PathBuf,
    #[arg(long, default_value = "heardright")]
    pub provider: String,
}

#[derive(Debug, Subcommand)]
pub enum BenchCommand {
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
    Playback {
        project: PathBuf,
        #[arg(long, default_value_t = 20)]
        runs: usize,
    },
}

#[derive(Debug, Subcommand)]
pub enum AnalyzeCommand {
    Local(PathCommand),
    Cloud(CloudAnalyzeArgs),
}

#[derive(Debug, Args)]
pub struct CloudAnalyzeArgs {
    pub project: PathBuf,
    #[arg(
        long,
        help = "'fake' (test-fixture adapter) or any other name, which resolves to the \
                default disabled provider — no real vendor ships yet"
    )]
    pub provider: String,
    #[arg(
        long,
        default_value = "frame-semantics",
        help = "frame-semantics|segment-semantics|video-index"
    )]
    pub capability: String,
    #[arg(
        long,
        help = "Asset to analyze, relative to the project; defaults to the first entry under \
                cache/proxies (the safe default)"
    )]
    pub target: Option<PathBuf>,
    #[arg(
        long,
        help = "Upload the registered source instead of a proxy; still refused unless the \
                project's cloud-config.json has upload_policy=source"
    )]
    pub use_source: bool,
}

#[derive(Debug, Subcommand)]
pub enum ReframeCommand {
    Plan(VariantPathCommand),
}

#[derive(Debug, Subcommand)]
pub enum ReviewCommand {
    Open(PathCommand),
    Select {
        project: PathBuf,
        #[arg(long)]
        variant: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum EvidenceCommand {
    Build(PathCommand),
}

#[derive(Debug, Subcommand)]
pub enum EditCommand {
    Candidates(PathCommand),
    Validate(VariantPathCommand),
    Render {
        project: PathBuf,
        #[arg(long)]
        variant: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum TranscriptCommand {
    Remap {
        project: PathBuf,
        #[arg(long)]
        variant: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct ShortsArgs {
    #[command(subcommand)]
    pub command: ShortsCommand,
}

#[derive(Debug, Subcommand)]
pub enum ShortsCommand {
    Propose {
        project: PathBuf,
        #[arg(long, default_value_t = 4)]
        count: u8,
    },
}

#[derive(Debug, Subcommand)]
pub enum FinishCommand {
    Validate(VariantPathCommand),
}

#[derive(Debug, Args)]
pub struct SlotArgs {
    #[command(subcommand)]
    pub command: SlotCommand,
}

#[derive(Debug, Subcommand)]
pub enum SlotCommand {
    Render { project: PathBuf, slot_id: String },
}

#[derive(Debug, Subcommand)]
pub enum RenderCommand {
    Preview(PathCommand),
    Final {
        project: PathBuf,
        #[arg(long)]
        preset: String,
        #[arg(
            long,
            help = "tight|natural; defaults to the reviewed selection, else natural"
        )]
        variant: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum PackageCommand {
    Social(PathCommand),
}

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    Otio(VariantPathCommand),
}

#[derive(Debug, Subcommand)]
pub enum ReceiptsCommand {
    /// Re-hash every recorded stage-receipt input/output against the bytes
    /// currently on disk and report any binding that no longer holds
    /// (hardening plan §10.4).
    Verify(PathCommand),
}

#[derive(Debug, Subcommand)]
pub enum PreferencesCommand {
    /// Learn only from current, hash-bound decision records across one or
    /// more reviewed projects and propose per-format recommendations. Never
    /// applies anything to a render — this command only ever writes a
    /// recommendations report (REV2 plan §15.7).
    Recommend {
        #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
        projects: Vec<PathBuf>,
        #[arg(
            long,
            help = "Where to write recommendations.json; required when more than one project is given, else defaults to <project>/feedback/preferences/recommendations.json"
        )]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum CloudCommand {
    /// Set or clear a project's explicit consent to run cloud analysis.
    /// Consent is `false` by default and there is no global switch — this
    /// is the only way it ever becomes `true` (REV2 plan §15.6).
    Consent {
        project: PathBuf,
        #[arg(long, help = "Omit to disable; pass to enable")]
        enable: bool,
    },
    /// Set a project's hard cloud-spend ceiling in USD. `0.0` (the default)
    /// blocks every cloud request outright.
    Budget {
        project: PathBuf,
        #[arg(long)]
        usd: f64,
    },
    /// Remove every currently-retained cloud upload/response for a project.
    /// Tombstones retention records and deletes their files; never touches
    /// the spend ledger.
    Delete(PathCommand),
}

#[derive(Debug, Subcommand)]
pub enum CapabilitiesCommand {
    /// List every capability declared by the canonical registry. Default
    /// output is a single JSON document matching `cutright.capability_list/v1`.
    List {
        /// Optional capability-id filter (exact match).
        #[arg(long)]
        id: Option<String>,
        /// Emit machine-readable JSON only (default; included for symmetry).
        #[arg(long, default_value_t = true)]
        json: bool,
    },
}

/// Args for `videoctl apply` (CR-V2-B2-023). The command reads a
/// `cutright.action_batch/v1` envelope from stdin (default) or from a file
/// and forwards it to [`video_project::ActionExecutor`]. The executor
/// owns every byte of validation, dry-run, and atomic apply logic; this
/// struct only carries transport flags.
#[derive(Debug, Args, Clone)]
pub struct ApplyArgs {
    /// Path to the action_batch JSON file. When absent, the command reads
    /// from stdin.
    #[arg(long, short = 'f')]
    pub file: Option<std::path::PathBuf>,
    /// Force reading from stdin even when a tty is detected.
    #[arg(long)]
    pub from_stdin: bool,
    /// Project directory the executor writes into. Defaults to the
    /// `CUTRIGHT_PROJECT_DIR` environment variable.
    #[arg(long, short = 'p')]
    pub project: Option<std::path::PathBuf>,
    /// Capability registry path. Defaults to
    /// `docs/dispatch/v2/source/capability-registry.json` in the repo.
    #[arg(long)]
    pub registry: Option<std::path::PathBuf>,
}
