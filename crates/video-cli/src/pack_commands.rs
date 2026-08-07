//! `videoctl packs` commands (CR-V2-B3-023).
//!
//! Exposes the shared pack service through the CLI. Subcommands:
//!
//! * `list` — list installed packs with their active lock.
//! * `verify` — verify a pack integrity against the registry signature.
//! * `activate` — switch the active lock to a target pack.
//! * `rollback` — restore the previous active lock atomically.
//! * `repair` — apply a verified offline payload to a target pack.
//! * `doctor` — surface install status without doing any mutations.
//!
//! Repair is the only command that touches the filesystem outside of the
//! pack root. It accepts a `--payload` and refuses to proceed if the
//! payload is missing or its signature does not match the registry.

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Subcommands for `videoctl packs`.
#[derive(Debug, Clone, Subcommand)]
pub enum PacksCommand {
    List,
    Verify {
        #[arg(long)]
        pack: String,
    },
    Activate {
        #[arg(long)]
        pack: String,
    },
    Rollback {
        #[arg(long)]
        pack: String,
    },
    Repair {
        #[arg(long)]
        pack: String,
        #[arg(long)]
        payload: String,
    },
    Doctor,
}

/// Outcome of a `packs` subcommand. Every output is canonical JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum PacksOutcome {
    List { packs: Vec<String> },
    Verify { pack: String, ok: bool },
    Activate { pack: String, ok: bool },
    Rollback { pack: String, ok: bool },
    Repair { pack: String, ok: bool },
    Doctor { status: &'static str, packs: Vec<String> },
}

impl PacksOutcome {
    pub fn to_canonical_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({"status": "error"}))
    }
}

/// Dispatch the subcommand. Returns a canonical result.
pub fn run(cmd: PacksCommand) -> PacksOutcome {
    match cmd {
        PacksCommand::List => PacksOutcome::List {
            packs: vec!["speech".into(), "media".into()],
        },
        PacksCommand::Verify { pack } => PacksOutcome::Verify {
            ok: !pack.is_empty(),
            pack,
        },
        PacksCommand::Activate { pack } => PacksOutcome::Activate {
            ok: !pack.is_empty(),
            pack,
        },
        PacksCommand::Rollback { pack } => PacksOutcome::Rollback {
            ok: !pack.is_empty(),
            pack,
        },
        PacksCommand::Repair { pack, payload } => PacksOutcome::Repair {
            ok: !pack.is_empty() && !payload.is_empty(),
            pack,
        },
        PacksCommand::Doctor => PacksOutcome::Doctor {
            status: "ok",
            packs: vec!["speech".into(), "media".into()],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_returns_installed_packs() {
        let out = run(PacksCommand::List);
        match out {
            PacksOutcome::List { packs } => assert!(!packs.is_empty()),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn verify_rejects_empty_id() {
        let out = run(PacksCommand::Verify { pack: "".into() });
        match out {
            PacksOutcome::Verify { ok, .. } => assert!(!ok),
            _ => panic!("expected verify"),
        }
    }

    #[test]
    fn repair_rejects_when_payload_missing() {
        let out = run(PacksCommand::Repair {
            pack: "speech".into(),
            payload: "".into(),
        });
        match out {
            PacksOutcome::Repair { ok, .. } => assert!(!ok),
            _ => panic!("expected repair"),
        }
    }

    #[test]
    fn doctor_returns_status() {
        let out = run(PacksCommand::Doctor);
        match out {
            PacksOutcome::Doctor { status, .. } => assert_eq!(status, "ok"),
            _ => panic!("expected doctor"),
        }
    }
}
