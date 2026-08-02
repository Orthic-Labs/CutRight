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
}

#[derive(Debug, Subcommand)]
pub enum AnalyzeCommand {
    Local(PathCommand),
    Cloud {
        project: PathBuf,
        #[arg(long)]
        provider: String,
    },
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
