//! A project's durable identity, independent of its human title and its
//! legacy `project_id`.
//!
//! REV2 plan §12.7: `video_project::init_project` (in `crates/video-project`,
//! out of scope for this change) derives `project_id` from the project
//! folder's name — `format!("project-{}", blake3::hash(project_name...))` —
//! so two differently-located projects that happen to share a folder name
//! collide. Fixing that generation is crate-side work this change does not
//! make (see the note at the bottom of this file). What Studio can do without
//! touching the crate is give every project a second, genuinely random
//! identity that Studio itself owns and never regenerates: a sidecar file,
//! `.cutright-studio/identity.json`, holding a `project_instance_id` created
//! once on first Studio access (new or pre-existing project, no distinction
//! needed) and never touched again — not on folder rename, not on relink.
//! `decision_contract.rs` binds decisions to this id in addition to the
//! legacy `project_id`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

const SCHEMA_VERSION: u32 = 1;
const IDENTITY_REL: &str = ".cutright-studio/identity.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIdentity {
    pub schema_version: u32,
    pub project_instance_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_project_id: Option<String>,
}

/// Resolve (creating on first use) a project's immutable instance id.
///
/// Uses `create_new` to open the destination file, which fails atomically if
/// another process already created it, so a concurrent caller cannot clobber
/// an identity that just came into existence — the loser simply re-reads the
/// winner's file. `legacy_project_id` is recorded only at creation time; it
/// has no effect once the identity already exists.
pub fn resolve(root: &Path, legacy_project_id: Option<&str>) -> Result<ProjectIdentity, String> {
    let path = root.join(IDENTITY_REL);
    if !path.is_file() {
        let identity = ProjectIdentity {
            schema_version: SCHEMA_VERSION,
            project_instance_id: format!("pin_{}", uuid::Uuid::new_v4()),
            created_at: chrono::Utc::now(),
            legacy_project_id: legacy_project_id.map(str::to_owned),
        };
        write_if_absent(&path, &identity)?;
    }
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

/// Create `path` with `value`'s JSON, but only if no one else has already.
///
/// Writes the full content to a uniquely named temp file in the same
/// directory first, then [`fs::hard_link`]s it onto the destination — a
/// destination that already exists makes the link fail with `AlreadyExists`
/// without disturbing what is there. This is deliberately not
/// `OpenOptions::create_new` directly on `path`: `create_new` makes the
/// destination's directory entry visible before its content is written, so a
/// concurrent reader that loses the race can observe a truncated (empty)
/// file instead of either "does not exist yet" or "fully written". Hard-link
/// only ever attaches a name to bytes that are already complete on disk.
fn write_if_absent(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');

    let temp = parent.join(format!(".identity.tmp-{}", uuid::Uuid::new_v4()));
    {
        let mut file =
            fs::File::create(&temp).map_err(|error| format!("{}: {error}", temp.display()))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("{}: {error}", temp.display()))?;
    }
    let result = fs::hard_link(&temp, path);
    let _ = fs::remove_file(&temp);
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn scratch_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cutright-project-identity-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn first_resolve_creates_a_random_instance_id_and_records_the_legacy_id() {
        let root = scratch_root();
        let identity = resolve(&root, Some("project-legacy-abc")).unwrap();
        assert!(identity.project_instance_id.starts_with("pin_"));
        assert_eq!(
            identity.legacy_project_id.as_deref(),
            Some("project-legacy-abc")
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn resolve_is_idempotent_and_never_regenerates() {
        let root = scratch_root();
        let first = resolve(&root, Some("project-legacy-abc")).unwrap();
        let second = resolve(&root, Some("a-different-legacy-id")).unwrap();
        assert_eq!(first.project_instance_id, second.project_instance_id);
        // The legacy id recorded at creation is not overwritten by a later,
        // different value passed to resolve — identity is immutable.
        assert_eq!(
            second.legacy_project_id.as_deref(),
            Some("project-legacy-abc")
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn two_freshly_created_projects_get_different_instance_ids() {
        let root_a = scratch_root();
        let root_b = scratch_root();
        let a = resolve(&root_a, None).unwrap();
        let b = resolve(&root_b, None).unwrap();
        assert_ne!(a.project_instance_id, b.project_instance_id);
        fs::remove_dir_all(&root_a).unwrap();
        fs::remove_dir_all(&root_b).unwrap();
    }
}

// Deferred crate-side work (REV2 §12.7), not made by this change because
// `crates/video-project` is out of scope here:
//   - `init_project` should generate a random `project_id` at creation
//     instead of hashing the folder name.
//   - `ProjectManifest`/the schema migration in `migrate_project` should gain
//     a first-class, immutable `project_instance_id` so it is not a Studio
//     sidecar file but part of the canonical project schema.
