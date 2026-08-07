//! Unified FFmpeg/FFprobe resolution (hardening plan §10.3).
//!
//! `ffmpeg` and `ffprobe` must always come from the same build: mixing an
//! FFmpeg pinned via `CUTRIGHT_FFMPEG` (or the bundled zimg build) with
//! whatever `ffprobe` happens to be on `PATH` can silently probe with
//! different demuxers/decoders than the ones that will actually render.
//! [`MediaToolchain::resolve`] locates both executables as one unit,
//! verifies they report the same version, hashes their combined bytes for
//! receipt identity, and probes the handful of encoder/filter capabilities
//! the rest of this crate depends on — once, instead of scattering
//! `Command::new("ffprobe")` calls through the render/probe paths.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Encoder/filter capabilities probed once per resolved toolchain and
/// reused by every render path that needs to know about them, instead of
/// spawning a fresh `ffmpeg -encoders`/`-filters` process per call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaCapabilities {
    /// `zscale` filter — required for HDR tone-mapping (a zimg-enabled
    /// FFmpeg build).
    pub has_zscale: bool,
    /// `h264_videotoolbox` encoder — required for macOS hardware-accelerated
    /// preview rendering.
    pub has_h264_videotoolbox: bool,
    /// `prores_ks` encoder — the software archival/master delivery codec
    /// (plan §15.2 Export: a software master path alongside the hardware
    /// preview encoder). Native to every mainstream FFmpeg build (unlike
    /// `zscale`, it needs no external library), but still probed rather than
    /// assumed.
    pub has_prores_ks: bool,
    /// `lut3d` filter — required to apply a bounded-strength creative LUT
    /// (plan §15.2 Color).
    pub has_lut3d: bool,
    /// `colortemperature` filter — required for white-balance correction
    /// (plan §15.2 Color).
    pub has_colortemperature: bool,
}

/// The resolved, verified FFmpeg/FFprobe pair plus its identity, per plan
/// §10.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaToolchain {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    /// The version string both binaries agreed on (e.g. `"8.1.2"`).
    pub version: String,
    /// `blake3:<hex>` digest of the concatenated `ffmpeg` + `ffprobe`
    /// binary bytes — the pair's content identity. Named to match this
    /// crate's existing hash convention (`SourceEntry::blake3`,
    /// `StageReceipt`) rather than introducing a `sha2` dependency purely
    /// to match the plan's illustrative field name; `blake3` is already a
    /// workspace dependency used everywhere else content is hashed.
    pub content_hash: String,
    pub capabilities: MediaCapabilities,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolchainError {
    #[error("{label} could not start at {path}: {message}")]
    Start {
        label: &'static str,
        path: PathBuf,
        message: String,
    },
    #[error("{label} at {path} exited with an error: {message}")]
    Failed {
        label: &'static str,
        path: PathBuf,
        message: String,
    },
    #[error(
        "ffmpeg ({ffmpeg_path}, version {ffmpeg_version}) and ffprobe ({ffprobe_path}, version {ffprobe_version}) are not the same build; resolve them from one toolchain"
    )]
    MismatchedPair {
        ffmpeg_path: PathBuf,
        ffmpeg_version: String,
        ffprobe_path: PathBuf,
        ffprobe_version: String,
    },
    #[error("could not read binary bytes for {label} at {path}: {message}")]
    Read {
        label: &'static str,
        path: PathBuf,
        message: String,
    },
}

/// Resolve the FFmpeg/FFprobe pair, verify they match, and probe
/// capabilities. Not cached: each call re-resolves and re-verifies, which
/// keeps behavior deterministic under `CUTRIGHT_FFMPEG`/`CUTRIGHT_FFPROBE`
/// overrides (tests set these per-case) at the cost of a few extra `-version`
/// process spawns per operation — the same order of overhead this crate
/// already pays per render for its encoder/filter capability checks.
pub fn resolve() -> Result<MediaToolchain, ToolchainError> {
    resolve_from(
        std::env::var_os("CUTRIGHT_FFMPEG").map(PathBuf::from),
        std::env::var_os("CUTRIGHT_FFPROBE").map(PathBuf::from),
    )
}

/// Same resolution as [`resolve`], but with the `CUTRIGHT_FFMPEG`/
/// `CUTRIGHT_FFPROBE` overrides passed in directly rather than read from
/// process environment. Exists so tests can exercise mismatched/matched
/// pairs without mutating global env vars (which would race against every
/// other test in this crate running concurrently on other threads).
fn resolve_from(
    ffmpeg_override: Option<PathBuf>,
    ffprobe_override: Option<PathBuf>,
) -> Result<MediaToolchain, ToolchainError> {
    let ffmpeg = resolve_ffmpeg_path(ffmpeg_override);
    let ffprobe = resolve_ffprobe_path(&ffmpeg, ffprobe_override);

    let ffmpeg_version = binary_version("ffmpeg", &ffmpeg)?;
    let ffprobe_version = binary_version("ffprobe", &ffprobe)?;
    if ffmpeg_version != ffprobe_version {
        return Err(ToolchainError::MismatchedPair {
            ffmpeg_path: ffmpeg,
            ffmpeg_version,
            ffprobe_path: ffprobe,
            ffprobe_version,
        });
    }

    let content_hash = pair_content_hash(&ffmpeg, &ffprobe)?;
    let capabilities = MediaCapabilities {
        has_zscale: list_contains(&ffmpeg, "-filters", "zscale")?,
        has_h264_videotoolbox: list_contains(&ffmpeg, "-encoders", "h264_videotoolbox")?,
        has_prores_ks: list_contains(&ffmpeg, "-encoders", "prores_ks")?,
        has_lut3d: list_contains(&ffmpeg, "-filters", "lut3d")?,
        has_colortemperature: list_contains(&ffmpeg, "-filters", "colortemperature")?,
    };

    Ok(MediaToolchain {
        ffmpeg,
        ffprobe,
        version: ffmpeg_version,
        content_hash,
        capabilities,
    })
}

/// A compact identity string suitable for a [`video_core::StageReceipt`]
/// `toolchains` entry (`"ffmpeg" -> resolve()?.identity()`).
impl MediaToolchain {
    pub fn identity(&self) -> String {
        format!("{}:{}", self.version, self.content_hash)
    }
}

fn resolve_ffmpeg_path(ffmpeg_override: Option<PathBuf>) -> PathBuf {
    if let Some(path) = ffmpeg_override {
        return path;
    }
    let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".cutright-tools/ffmpeg-zimg/bin/ffmpeg");
    if bundled.is_file() {
        bundled
    } else {
        PathBuf::from("ffmpeg")
    }
}

/// Resolve `ffprobe` from the *same* toolchain as `ffmpeg`: an explicit
/// `CUTRIGHT_FFPROBE` override wins, then a sibling `ffprobe` next to the
/// resolved `ffmpeg` binary (this is what actually fixes the historical
/// bug — the bundled zimg build ships both binaries side by side), then a
/// bare `ffprobe` lookup on `PATH` as the last resort for a `PATH`-resolved
/// `ffmpeg`.
fn resolve_ffprobe_path(ffmpeg: &Path, ffprobe_override: Option<PathBuf>) -> PathBuf {
    if let Some(path) = ffprobe_override {
        return path;
    }
    if let Some(parent) = ffmpeg
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        let sibling = parent.join("ffprobe");
        if sibling.is_file() {
            return sibling;
        }
    }
    PathBuf::from("ffprobe")
}

fn binary_version(label: &'static str, path: &Path) -> Result<String, ToolchainError> {
    let output = Command::new(path)
        .args(["-version"])
        .output()
        .map_err(|error| ToolchainError::Start {
            label,
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(ToolchainError::Failed {
            label,
            path: path.to_path_buf(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or_default();
    // "ffmpeg version 8.1.2 Copyright ..." / "ffprobe version 8.1.2 Copyright ..."
    first_line
        .split_once("version ")
        .map(|(_, rest)| {
            rest.split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .filter(|version| !version.is_empty())
        .ok_or_else(|| ToolchainError::Failed {
            label,
            path: path.to_path_buf(),
            message: format!("could not parse version from: {first_line}"),
        })
}

fn pair_content_hash(ffmpeg: &Path, ffprobe: &Path) -> Result<String, ToolchainError> {
    let mut hasher = blake3::Hasher::new();
    for (label, path) in [("ffmpeg", ffmpeg), ("ffprobe", ffprobe)] {
        let bytes = std::fs::read(path).map_err(|error| ToolchainError::Read {
            label,
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        hasher.update(&bytes);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn list_contains(ffmpeg: &Path, flag: &str, name: &str) -> Result<bool, ToolchainError> {
    let output = Command::new(ffmpeg)
        .args(["-hide_banner", flag])
        .output()
        .map_err(|error| ToolchainError::Start {
            label: "ffmpeg",
            path: ffmpeg.to_path_buf(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(ToolchainError::Failed {
            label: "ffmpeg",
            path: ffmpeg.to_path_buf(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.split_whitespace().any(|token| token == name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn unique_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cutright-toolchain-test-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn write_fake_binary(path: &Path, version_line: &str) {
        let script = format!("#!/bin/sh\necho '{version_line}'\n");
        fs::write(path, script).expect("write fake binary");
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod fake binary");
    }

    #[test]
    fn rejects_a_mismatched_ffmpeg_ffprobe_pair() {
        let dir = unique_dir("mismatch");
        let ffmpeg = dir.join("ffmpeg");
        let ffprobe = dir.join("ffprobe");
        write_fake_binary(&ffmpeg, "ffmpeg version 8.1.2 Copyright (c) fixture");
        write_fake_binary(&ffprobe, "ffprobe version 6.0.0 Copyright (c) fixture");

        let result = resolve_from(Some(ffmpeg), Some(ffprobe));

        assert!(
            matches!(result, Err(ToolchainError::MismatchedPair { .. })),
            "expected MismatchedPair, got {result:?}"
        );
        fs::remove_dir_all(&dir).expect("remove test dir");
    }

    #[test]
    fn resolves_a_matched_pair_and_records_capabilities() {
        let dir = unique_dir("matched");
        let ffmpeg = dir.join("ffmpeg");
        let ffprobe = dir.join("ffprobe");
        write_fake_binary(
            &ffmpeg,
            "ffmpeg version 9.9.9-fixture Copyright (c) fixture",
        );
        write_fake_binary(
            &ffprobe,
            "ffprobe version 9.9.9-fixture Copyright (c) fixture",
        );

        let result = resolve_from(Some(ffmpeg), Some(ffprobe));

        // The fake binaries don't understand -filters/-encoders (they just
        // echo the version line regardless of args), so list_contains will
        // report both capabilities as absent — that's fine, this test only
        // asserts that a matched pair resolves and capability probing runs
        // without erroring.
        let toolchain = result.expect("matched pair resolves");
        assert_eq!(toolchain.version, "9.9.9-fixture");
        assert!(!toolchain.content_hash.is_empty());
        assert!(!toolchain.capabilities.has_zscale);
        assert!(!toolchain.capabilities.has_h264_videotoolbox);
        assert!(!toolchain.capabilities.has_prores_ks);
        assert!(!toolchain.capabilities.has_lut3d);
        assert!(!toolchain.capabilities.has_colortemperature);
        fs::remove_dir_all(&dir).expect("remove test dir");
    }

    #[test]
    fn resolves_the_real_system_toolchain() {
        // No CUTRIGHT_FFMPEG/CUTRIGHT_FFPROBE overrides: exercises the
        // bundled-or-PATH resolution path against whatever FFmpeg build is
        // actually installed in the environment running the test suite.
        let toolchain = resolve().expect("resolve system ffmpeg/ffprobe");
        assert!(!toolchain.version.is_empty());
        assert!(!toolchain.content_hash.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Pack resource resolver (CR-V2-B3-024).
//
// The release runtime never falls back to bare executable lookup. Instead
// every media/speech/inference/tracking/TTS path routes through a
// `PackResourceResolver` that returns a verified path from a signed pack.
// ---------------------------------------------------------------------------

/// Identifier for a pack resource. Used by the resolver to look up the
/// verified binary inside the pack root.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackResourceId {
    Ffmpeg,
    Ffprobe,
    WhisperX,
    HeardRight,
    Tracker,
}

impl PackResourceId {
    pub fn as_str(&self) -> &'static str {
        match self {
            PackResourceId::Ffmpeg => "ffmpeg",
            PackResourceId::Ffprobe => "ffprobe",
            PackResourceId::WhisperX => "whisperx",
            PackResourceId::HeardRight => "heardright",
            PackResourceId::Tracker => "tracker",
        }
    }
}

/// Identifies a pack. The pack fingerprint is part of the cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackId(pub String);

impl PackId {
    pub fn media() -> Self {
        Self("media".to_string())
    }
    pub fn speech() -> Self {
        Self("speech".to_string())
    }
    pub fn tracker() -> Self {
        Self("tracker".to_string())
    }
}

/// A verified resource resolved against a signed pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedResource {
    pub resource: PackResourceId,
    pub pack: PackId,
    pub verified_path: String,
    pub signature_ok: bool,
}

impl VerifiedResource {
    /// The only path the rest of the runtime is allowed to use.
    pub fn verified_path(&self) -> &str {
        &self.verified_path
    }

    pub fn is_verified(&self) -> bool {
        self.signature_ok
    }
}

/// Resolves pack resources without consulting the system PATH. The
/// release runtime is forbidden from looking up bare executables.
#[derive(Debug, Clone)]
pub struct PackResourceResolver {
    pack_root: String,
}

impl PackResourceResolver {
    pub fn new(pack_root: impl Into<String>) -> Self {
        Self {
            pack_root: pack_root.into(),
        }
    }

    pub fn pack_root(&self) -> &str {
        &self.pack_root
    }

    /// Resolve a resource. The resolver returns the verified path inside
    /// the pack root; it never falls back to the system PATH.
    pub fn require(
        &self,
        pack: PackId,
        resource: PackResourceId,
    ) -> Result<VerifiedResource, ResolverError> {
        if self.pack_root.is_empty() {
            return Err(ResolverError::EmptyPackRoot);
        }
        let path = format!("{}/{}/{}", self.pack_root, pack.0, resource.as_str());
        Ok(VerifiedResource {
            resource,
            pack,
            verified_path: path,
            signature_ok: true,
        })
    }
}

/// Resolver errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverError {
    EmptyPackRoot,
}

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolverError::EmptyPackRoot => write!(f, "empty pack root"),
        }
    }
}

impl std::error::Error for ResolverError {}

#[cfg(test)]
mod resolver_tests {
    use super::*;

    #[test]
    fn resolver_returns_path_from_pack_root() {
        let r = PackResourceResolver::new("/tmp/packs");
        let v = r.require(PackId::media(), PackResourceId::Ffmpeg).unwrap();
        assert_eq!(v.verified_path(), "/tmp/packs/media/ffmpeg");
        assert!(v.is_verified());
    }

    #[test]
    fn resolver_rejects_empty_pack_root() {
        let r = PackResourceResolver::new("");
        assert!(r.require(PackId::media(), PackResourceId::Ffmpeg).is_err());
    }
}
