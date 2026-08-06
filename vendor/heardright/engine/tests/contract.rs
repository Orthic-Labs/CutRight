use std::sync::{Mutex, MutexGuard, OnceLock};

fn settings_override_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/contract_sections/section01.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/contract_sections/section02.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/contract_sections/section03.rs"
));
