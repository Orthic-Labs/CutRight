//! Sidecar-side crash capture — the ASR engine's own postmortem.
//!
//! The shell (`heardright_next`) can only dump ITS OWN process when it notices
//! the sidecar died; by then the sidecar is gone and undumpable from outside.
//! So the engine installs its own panic hook and writes a real postmortem of
//! itself into the SAME `app_data/crash` directory the shell exports and
//! prunes. Filenames use the shell's retention prefixes (`dump_` / `crash_`)
//! so the shell's `crash_dump::prune_crash_dir` manages engine artifacts too.
//!
//! The engine builds with `panic = "abort"`, so the hook runs before abort
//! (like the shell's). MUST be panic-safe: no `unwrap`/`expect`, every step
//! best-effort. Dumps/logs carry process/thread state only — no transcript
//! text — so they need no separate redaction before travelling in the export.

use std::io::Write;
use std::path::PathBuf;

fn crash_dir() -> PathBuf {
    crate::settings::app_data_root().join("crash")
}

fn unix_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Install the sidecar panic hook. Chains to the previous hook so tracing's
/// stderr behaviour is preserved. Call once, early in `main`.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort and must NEVER panic itself (a panic in the hook aborts
        // immediately with no report). Every step is fallible-but-ignored.
        let stamp = unix_ts();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_string());
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let thread = std::thread::current()
            .name()
            .unwrap_or("<unnamed>")
            .to_string();
        let backtrace = std::backtrace::Backtrace::force_capture();

        let dir = crash_dir();
        if std::fs::create_dir_all(&dir).is_ok() {
            let path = dir.join(format!("crash_sidecar_{stamp}.log"));
            if let Ok(mut f) = std::fs::File::create(&path) {
                let _ = writeln!(f, "HeardRight sidecar crash report");
                let _ = writeln!(f, "version: {}", env!("CARGO_PKG_VERSION"));
                let _ = writeln!(f, "unix_time: {stamp}");
                let _ = writeln!(f, "thread: {thread}");
                let _ = writeln!(f, "location: {loc}");
                let _ = writeln!(f, "message: {msg}");
                let _ = writeln!(f, "\nbacktrace:\n{backtrace}");
            }
            // Windows: a real minidump of the crashing sidecar itself.
            #[cfg(target_os = "windows")]
            windows_dump::write(&dir, stamp);
        }
        tracing::error!("sidecar panic thread={thread} loc={loc} msg={msg:?}");

        previous(info);
    }));
}

#[cfg(target_os = "windows")]
mod windows_dump {
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;

    use windows_sys::Win32::System::Diagnostics::Debug::{
        MiniDumpNormal, MiniDumpWithThreadInfo, MiniDumpWriteDump,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId};

    /// Dump the current (sidecar) process/thread. `MiniDumpNormal |
    /// MiniDumpWithThreadInfo` gives modules + thread stacks/contexts without
    /// the full-memory bloat — enough for postmortem triage.
    pub(super) fn write(dir: &Path, stamp: u64) {
        let path = dir.join(format!("dump_sidecar_{stamp}.dmp"));
        let Ok(file) = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
        else {
            return;
        };
        let handle = file.as_raw_handle() as *mut core::ffi::c_void;
        let ok = unsafe {
            MiniDumpWriteDump(
                GetCurrentProcess(),
                GetCurrentProcessId(),
                handle,
                MiniDumpNormal | MiniDumpWithThreadInfo,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        drop(file);
        if ok == 0 {
            let _ = std::fs::remove_file(&path);
        }
    }
}
