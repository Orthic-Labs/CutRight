//! Cross-platform test helpers. Compiled on every OS (unlike `apple_foundation`,
//! which is macOS-gated) so Windows `cargo test` can use the same env lock.

static TEST_ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

/// Serialize tests that mutate process env vars. `std::env::set_var` is
/// process-global, so concurrent tests that touch env state race without this.
/// Poisoning is ignored: a panicked test holding the lock must not cascade.
pub(crate) fn test_env_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
